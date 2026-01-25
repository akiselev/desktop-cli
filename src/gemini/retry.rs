use crate::error::{GeminiError, GeminiResult};
use crate::gemini::client::GeminiClient;
use crate::gemini::schema::{AlternativeElement, ElementDetectionResult};
use std::time::Duration;
use tokio::time::sleep;

/// Retry strategy configuration
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// No retries - fail immediately
    None,
    /// Basic retry with exponential backoff
    Basic { max_retries: u32 },
    /// Advanced retry with fuzzy matching and disambiguation
    Advanced {
        max_retries: u32,
        fuzzy_threshold: f32, // 0-1, minimum similarity score
        enable_disambiguation: bool,
    },
}

impl RetryStrategy {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(Self::None),
            "basic" => Some(Self::Basic { max_retries: 3 }),
            "advanced" => Some(Self::Advanced {
                max_retries: 5,
                fuzzy_threshold: 0.7,
                enable_disambiguation: true,
            }),
            _ => None,
        }
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::Advanced {
            max_retries: 5,
            fuzzy_threshold: 0.7,
            enable_disambiguation: true,
        }
    }
}

/// Execute element detection with retry logic
pub async fn detect_element_with_retry(
    client: &GeminiClient,
    instruction: &str,
    screenshot_base64: &str,
    strategy: &RetryStrategy,
) -> GeminiResult<ElementDetectionResult> {
    match strategy {
        RetryStrategy::None => client.detect_element(instruction, screenshot_base64).await,

        RetryStrategy::Basic { max_retries } => {
            execute_with_basic_retry(client, instruction, screenshot_base64, *max_retries).await
        }

        RetryStrategy::Advanced {
            max_retries,
            fuzzy_threshold,
            enable_disambiguation,
        } => {
            execute_with_advanced_retry(
                client,
                instruction,
                screenshot_base64,
                *max_retries,
                *fuzzy_threshold,
                *enable_disambiguation,
            )
            .await
        }
    }
}

/// Basic retry with exponential backoff
async fn execute_with_basic_retry(
    client: &GeminiClient,
    instruction: &str,
    screenshot_base64: &str,
    max_retries: u32,
) -> GeminiResult<ElementDetectionResult> {
    let mut attempt = 0;

    loop {
        match client.detect_element(instruction, screenshot_base64).await {
            Ok(result) if result.element_found => {
                tracing::info!("Element found on attempt {}", attempt + 1);
                return Ok(result);
            }
            Ok(result) => {
                tracing::warn!("Element not found on attempt {}", attempt + 1);
                if attempt >= max_retries {
                    return Ok(result); // Return the last result even if not found
                }
            }
            Err(GeminiError::RateLimited { retry_after_secs }) => {
                tracing::warn!("Rate limited, waiting {} seconds", retry_after_secs);
                sleep(Duration::from_secs(retry_after_secs)).await;
                continue; // Don't count rate limits as a retry
            }
            Err(e) => {
                if attempt >= max_retries {
                    return Err(e);
                }
                tracing::warn!("Attempt {} failed: {}, retrying...", attempt + 1, e);
            }
        }

        attempt += 1;

        // Exponential backoff: 1s, 2s, 4s, 8s
        let delay_secs = 2u64.pow(attempt.min(4));
        tracing::debug!("Waiting {} seconds before retry", delay_secs);
        sleep(Duration::from_secs(delay_secs)).await;
    }
}

/// Advanced retry with fuzzy matching, disambiguation, and context-aware prompts
async fn execute_with_advanced_retry(
    client: &GeminiClient,
    instruction: &str,
    screenshot_base64: &str,
    max_retries: u32,
    fuzzy_threshold: f32,
    enable_disambiguation: bool,
) -> GeminiResult<ElementDetectionResult> {
    let mut attempt = 0;
    let context_hints = vec![
        "",
        "in the top half of the screen",
        "in the bottom half",
        "on the left side",
        "on the right side",
    ];

    loop {
        // Build instruction with context hint if we're retrying
        let enhanced_instruction = if attempt > 0 && attempt < context_hints.len() as u32 {
            format!("{} {}", instruction, context_hints[attempt as usize])
        } else {
            instruction.to_string()
        };

        match client
            .detect_element(&enhanced_instruction, screenshot_base64)
            .await
        {
            Ok(result) if result.element_found => {
                // Check confidence threshold
                if result.confidence >= fuzzy_threshold {
                    tracing::info!(
                        "Element '{}' found with confidence {} on attempt {}",
                        result.label,
                        result.confidence,
                        attempt + 1
                    );
                    return Ok(result);
                }

                tracing::warn!(
                    "Element found but confidence {} below threshold {}",
                    result.confidence,
                    fuzzy_threshold
                );

                // Try fuzzy matching with alternatives
                if !result.alternatives.is_empty() {
                    if let Some(alt) = find_best_alternative(&result, fuzzy_threshold) {
                        tracing::info!(
                            "Using alternative element '{}' with confidence {}",
                            alt.label,
                            alt.confidence
                        );
                        return Ok(ElementDetectionResult {
                            element_found: true,
                            label: alt.label.clone(),
                            bounding_box: alt.bounding_box.clone(),
                            action_type: result.action_type.clone(),
                            action_params: result.action_params.clone(),
                            confidence: alt.confidence,
                            alternatives: vec![],
                        });
                    }
                }

                // If disambiguation is enabled and we have multiple candidates
                if enable_disambiguation && !result.alternatives.is_empty() && attempt == 0 {
                    tracing::info!(
                        "Multiple candidates found, returning for disambiguation: {} alternatives",
                        result.alternatives.len()
                    );
                    return Ok(result); // Let the caller handle disambiguation
                }
            }

            Ok(result) => {
                tracing::warn!("Element not found on attempt {}", attempt + 1);
                if attempt >= max_retries {
                    return Ok(result); // Return the last result
                }
            }

            Err(GeminiError::RateLimited { retry_after_secs }) => {
                tracing::warn!("Rate limited, waiting {} seconds", retry_after_secs);
                sleep(Duration::from_secs(retry_after_secs)).await;
                continue; // Don't count rate limits as a retry
            }

            Err(e) => {
                if attempt >= max_retries {
                    return Err(e);
                }
                tracing::warn!("Attempt {} failed: {}, retrying...", attempt + 1, e);
            }
        }

        attempt += 1;

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s
        let delay_secs = 2u64.pow(attempt.min(5));
        tracing::debug!("Waiting {} seconds before retry", delay_secs);
        sleep(Duration::from_secs(delay_secs)).await;
    }
}

/// Find the best alternative element that meets the fuzzy threshold
fn find_best_alternative(
    result: &ElementDetectionResult,
    threshold: f32,
) -> Option<&AlternativeElement> {
    result
        .alternatives
        .iter()
        .filter(|alt| alt.confidence >= threshold)
        .max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_strategy_from_str() {
        assert!(matches!(
            RetryStrategy::from_str("none"),
            Some(RetryStrategy::None)
        ));
        assert!(matches!(
            RetryStrategy::from_str("basic"),
            Some(RetryStrategy::Basic { .. })
        ));
        assert!(matches!(
            RetryStrategy::from_str("advanced"),
            Some(RetryStrategy::Advanced { .. })
        ));
        assert!(RetryStrategy::from_str("invalid").is_none());
    }

    #[test]
    fn test_find_best_alternative() {
        let result = ElementDetectionResult {
            element_found: true,
            label: "Primary".to_string(),
            bounding_box: vec![],
            action_type: "click".to_string(),
            action_params: None,
            confidence: 0.5,
            alternatives: vec![
                AlternativeElement {
                    label: "Alt1".to_string(),
                    bounding_box: vec![],
                    confidence: 0.8,
                },
                AlternativeElement {
                    label: "Alt2".to_string(),
                    bounding_box: vec![],
                    confidence: 0.9,
                },
                AlternativeElement {
                    label: "Alt3".to_string(),
                    bounding_box: vec![],
                    confidence: 0.6,
                },
            ],
        };

        let best = find_best_alternative(&result, 0.7);
        assert!(best.is_some());
        assert_eq!(best.unwrap().confidence, 0.9);

        let none = find_best_alternative(&result, 0.95);
        assert!(none.is_none());
    }
}
