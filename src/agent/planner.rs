//! LLM-based action planner using Gemini

use crate::agent::types::{AgentAction, PlannerResponse};
use crate::error::{GeminiError, GeminiResult};
use crate::gemini::schema::{
    Content, GeminiRequest, GeminiResponse, GenerationConfig, InlineData, Part,
};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Planner that uses Gemini to decide what action to take
pub struct AgentPlanner {
    client: Client,
    api_key: String,
    model: String,
}

impl AgentPlanner {
    pub fn new(api_key: String, model: String) -> GeminiResult<Self> {
        if api_key.is_empty() {
            return Err(GeminiError::AuthError);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(GeminiError::RequestFailed)?;

        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    /// Plan the next action based on current state
    pub async fn plan_next_action(
        &self,
        goal: &str,
        screenshot_base64: Option<&str>,
        ui_summary: Option<&str>,
        history: &[String],
    ) -> GeminiResult<PlannerResponse> {
        let request = self.build_planning_request(goal, screenshot_base64, ui_summary, history);
        let response = self.send_request(&request).await?;
        self.parse_planning_response(response)
    }

    fn build_planning_request(
        &self,
        goal: &str,
        screenshot_base64: Option<&str>,
        ui_summary: Option<&str>,
        history: &[String],
    ) -> GeminiRequest {
        let mut parts = Vec::new();

        // Build the system/context prompt
        let mut prompt = format!(
            r#"You are an automation agent controlling a Windows desktop application using UI Automation.

## YOUR GOAL
{goal}

## AVAILABLE ACTIONS
You can perform these actions (respond with exactly one):

1. **click** - Click on an element
   - target: Use the EXACT element name from the UI STATE below (e.g., "File", "Save", "OK")
   - click_type: "left" (default), "right", or "double"

2. **type** - Type text
   - text: The text to type
   - target: (optional) Element name of input field to focus first

3. **keys** - Send keyboard shortcut
   - keys: Key combination like "enter", "tab", "ctrl+s", "alt+f4", "alt+f" (for File menu)

4. **scroll** - Scroll the view
   - direction: "up" or "down"
   - amount: Number of scroll notches (default 3)

5. **wait** - Wait briefly
   - ms: Milliseconds to wait (default 500)

6. **done** - Goal achieved!
   - reason: Explain what was accomplished

7. **fail** - Goal cannot be achieved
   - reason: Explain why it's impossible

## RESPONSE FORMAT
Respond with JSON:
```json
{{
  "reasoning": "Brief analysis of current state and why this action",
  "action": "click|type|keys|scroll|wait|done|fail",
  ... action-specific fields ...
}}
```

## CRITICAL: ELEMENT TARGETING
- **ALWAYS use exact element names from the UI STATE section below**
- Look for elements with names in quotes like [Button] "Save" or [MenuItem] "File"
- For click targets, use just the name: "Save", "File", "OK" - NOT "Save button" or "the File menu"
- The UI STATE shows real accessibility tree elements - use those names exactly
- Keyboard shortcuts (alt+f for File menu) are often more reliable than clicking

## GUIDELINES
- Examine the UI STATE carefully - it shows all interactive elements
- Take ONE action at a time - you'll see the result and can continue
- For menus: try "alt+f" for File, "alt+e" for Edit, etc.
- For text input, click the field first if not focused
- If an element isn't in UI STATE, it may not be accessible - try keyboard navigation
- If stuck after 3 attempts, try a completely different approach
- Call "done" when the goal is clearly achieved
- Call "fail" only when truly impossible (not just difficult)
"#
        );

        // Add UI summary if available
        if let Some(summary) = ui_summary {
            prompt.push_str(&format!(
                r#"
## CURRENT UI STATE (Accessibility Tree Summary)
```
{summary}
```
"#
            ));
        }

        // Add history if any
        if !history.is_empty() {
            prompt.push_str("\n## PREVIOUS ACTIONS\n");
            for (i, action) in history.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, action));
            }
            prompt.push_str("\nContinue from where you left off.\n");
        }

        prompt.push_str("\nAnalyze the current state and decide the next action:");

        parts.push(Part::Text { text: prompt });

        // Add screenshot if available
        if let Some(img_data) = screenshot_base64 {
            parts.push(Part::InlineData {
                inline_data: InlineData {
                    mime_type: "image/png".to_string(),
                    data: img_data.to_string(),
                },
            });
        }

        GeminiRequest {
            contents: vec![Content { parts }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
                response_schema: self.planning_schema(),
            },
        }
    }

    fn planning_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "reasoning": {
                    "type": "string",
                    "description": "Brief reasoning about current state and chosen action"
                },
                "action": {
                    "type": "string",
                    "enum": ["click", "type", "keys", "scroll", "wait", "done", "fail"],
                    "description": "The action to perform"
                },
                "target": {
                    "type": "string",
                    "description": "For click: element description or 'x,y' coords. For type: optional field to focus."
                },
                "click_type": {
                    "type": "string",
                    "enum": ["left", "right", "double"],
                    "description": "Type of click (default: left)"
                },
                "text": {
                    "type": "string",
                    "description": "For type action: the text to type"
                },
                "keys": {
                    "type": "string",
                    "description": "For keys action: key combination like 'ctrl+s', 'enter'"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down"],
                    "description": "For scroll action: direction to scroll"
                },
                "amount": {
                    "type": "integer",
                    "description": "For scroll action: number of notches"
                },
                "ms": {
                    "type": "integer",
                    "description": "For wait action: milliseconds to wait"
                },
                "reason": {
                    "type": "string",
                    "description": "For done/fail actions: explanation"
                }
            },
            "required": ["reasoning", "action"]
        })
    }

    async fn send_request(&self, request: &GeminiRequest) -> GeminiResult<GeminiResponse> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE, self.model, self.api_key
        );

        tracing::debug!("Sending planning request to Gemini");

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(GeminiError::RequestFailed)?;

        let status = response.status();

        if !status.is_success() {
            if status.as_u16() == 429 {
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
            .map_err(|e| GeminiError::InvalidSchema(format!("Failed to parse response: {}", e)))?;

        if let Some(usage) = &gemini_response.usage_metadata {
            tracing::debug!(
                "Planner tokens: {} prompt, {} completion",
                usage.prompt_token_count,
                usage.candidates_token_count
            );
        }

        Ok(gemini_response)
    }

    fn parse_planning_response(&self, response: GeminiResponse) -> GeminiResult<PlannerResponse> {
        if response.candidates.is_empty() {
            return Err(GeminiError::InvalidSchema("No candidates".to_string()));
        }

        let candidate = &response.candidates[0];
        if candidate.content.parts.is_empty() {
            return Err(GeminiError::InvalidSchema("No parts".to_string()));
        }

        let json_text = &candidate.content.parts[0].text;
        tracing::debug!("Planner response: {}", json_text);

        // Parse the raw JSON first
        let raw: Value = serde_json::from_str(json_text).map_err(|e| {
            GeminiError::InvalidSchema(format!("JSON parse error: {}. Text: {}", e, json_text))
        })?;

        // Extract fields
        let reasoning = raw["reasoning"]
            .as_str()
            .unwrap_or("No reasoning provided")
            .to_string();

        let action_type = raw["action"]
            .as_str()
            .ok_or_else(|| GeminiError::InvalidSchema("Missing 'action' field".to_string()))?;

        // Build the appropriate action
        let action = match action_type {
            "click" => AgentAction::Click {
                target: raw["target"].as_str().unwrap_or("").to_string(),
                click_type: raw["click_type"].as_str().unwrap_or("left").to_string(),
            },
            "type" => AgentAction::Type {
                text: raw["text"].as_str().unwrap_or("").to_string(),
                target: raw["target"].as_str().map(|s| s.to_string()),
            },
            "keys" => AgentAction::Keys {
                keys: raw["keys"].as_str().unwrap_or("").to_string(),
            },
            "scroll" => AgentAction::Scroll {
                direction: raw["direction"].as_str().unwrap_or("down").to_string(),
                amount: raw["amount"].as_i64().unwrap_or(3) as i32,
            },
            "wait" => AgentAction::Wait {
                ms: raw["ms"].as_u64().unwrap_or(500),
            },
            "done" => AgentAction::Done {
                reason: raw["reason"]
                    .as_str()
                    .unwrap_or("Goal achieved")
                    .to_string(),
            },
            "fail" => AgentAction::Fail {
                reason: raw["reason"]
                    .as_str()
                    .unwrap_or("Cannot complete goal")
                    .to_string(),
            },
            _ => {
                return Err(GeminiError::InvalidSchema(format!(
                    "Unknown action: {}",
                    action_type
                )));
            }
        };

        Ok(PlannerResponse { reasoning, action })
    }
}
