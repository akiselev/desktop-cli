use crate::error::{GeminiError, GeminiResult};
use crate::gemini::schema::{
    build_element_detection_request, ElementDetectionResult, GeminiRequest, GeminiResponse,
};
use reqwest::Client;
use std::time::Duration;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Gemini API client for making structured output requests
#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    /// Create a new Gemini client
    pub fn new(api_key: String, model: String) -> GeminiResult<Self> {
        if api_key.is_empty() {
            return Err(GeminiError::AuthError);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(GeminiError::RequestFailed)?;

        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    /// Detect an element in a screenshot using natural language instruction
    pub async fn detect_element(
        &self,
        instruction: &str,
        screenshot_base64: &str,
    ) -> GeminiResult<ElementDetectionResult> {
        let request = build_element_detection_request(instruction, screenshot_base64, &self.model);
        let response = self.send_request(&request).await?;
        self.parse_element_detection_response(response)
    }

    /// Send a request to the Gemini API
    async fn send_request(&self, request: &GeminiRequest) -> GeminiResult<GeminiResponse> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE, self.model, self.api_key
        );

        tracing::debug!("Sending request to Gemini API: {}", self.model);

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(GeminiError::RequestFailed)?;

        let status = response.status();

        if !status.is_success() {
            // Handle specific error status codes
            if status.as_u16() == 429 {
                // Rate limited - try to extract retry-after header
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);

                return Err(GeminiError::RateLimited {
                    retry_after_secs: retry_after,
                });
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(GeminiError::ApiError {
                status: status.as_u16(),
                message: error_body,
            });
        }

        let gemini_response = response
            .json::<GeminiResponse>()
            .await
            .map_err(|e| {
                GeminiError::InvalidSchema(format!("Failed to parse Gemini response: {}", e))
            })?;

        // Log token usage if available
        if let Some(usage) = &gemini_response.usage_metadata {
            tracing::debug!(
                "Gemini API usage: {} prompt tokens, {} completion tokens, {} total",
                usage.prompt_token_count,
                usage.candidates_token_count,
                usage.total_token_count
            );
        }

        Ok(gemini_response)
    }

    /// Parse element detection result from Gemini response
    fn parse_element_detection_response(
        &self,
        response: GeminiResponse,
    ) -> GeminiResult<ElementDetectionResult> {
        if response.candidates.is_empty() {
            return Err(GeminiError::InvalidSchema(
                "No candidates in response".to_string(),
            ));
        }

        let candidate = &response.candidates[0];
        if candidate.content.parts.is_empty() {
            return Err(GeminiError::InvalidSchema(
                "No parts in candidate content".to_string(),
            ));
        }

        let json_text = &candidate.content.parts[0].text;

        let result: ElementDetectionResult = serde_json::from_str(json_text).map_err(|e| {
            GeminiError::InvalidSchema(format!(
                "Failed to parse element detection result: {}. JSON: {}",
                e, json_text
            ))
        })?;

        Ok(result)
    }

    /// Get the model name being used
    pub fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation_fails_without_api_key() {
        let result = GeminiClient::new("".to_string(), "gemini-3-flash-preview".to_string());
        assert!(matches!(result, Err(GeminiError::AuthError)));
    }

    #[test]
    fn test_client_creation_succeeds_with_api_key() {
        let result = GeminiClient::new(
            "test_api_key".to_string(),
            "gemini-3-flash-preview".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_element_detection_response() {
        let json_response = r#"{
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"element_found\": true, \"label\": \"Save\", \"bounding_box\": [100, 200, 150, 300], \"action_type\": \"click\", \"confidence\": 0.95, \"alternatives\": []}"
                    }]
                }
            }]
        }"#;

        let response: GeminiResponse = serde_json::from_str(json_response).unwrap();
        let client =
            GeminiClient::new("test".to_string(), "gemini-3-flash-preview".to_string()).unwrap();
        let result = client.parse_element_detection_response(response).unwrap();

        assert!(result.element_found);
        assert_eq!(result.label, "Save");
        assert_eq!(result.confidence, 0.95);
    }
}
