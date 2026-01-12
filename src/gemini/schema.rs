use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Gemini API request for element detection with structured outputs
#[derive(Debug, Clone, Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GenerationConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineData {
    pub mime_type: String,
    pub data: String, // base64-encoded image data
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    pub response_mime_type: String,
    #[serde(rename = "responseSchema")]
    pub response_schema: Value,
}

/// Gemini API response
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata", default)]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: ContentResponse,
    #[serde(rename = "finishReason", default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentResponse {
    pub parts: Vec<PartResponse>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PartResponse {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount", default)]
    pub total_token_count: u32,
}

/// Element detection result from Gemini (structured output)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ElementDetectionResult {
    pub element_found: bool,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub bounding_box: Vec<f32>, // [y_min, x_min, y_max, x_max] in 0-1000 range
    #[serde(default)]
    pub action_type: String, // "click", "type", "scroll", etc.
    #[serde(default)]
    pub action_params: Option<Value>,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub alternatives: Vec<AlternativeElement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlternativeElement {
    pub label: String,
    pub bounding_box: Vec<f32>,
    pub confidence: f32,
}

/// Build the response schema for element detection
pub fn element_detection_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "element_found": {
                "type": "boolean",
                "description": "Whether the requested element was found"
            },
            "label": {
                "type": "string",
                "description": "Descriptive name of the found element"
            },
            "bounding_box": {
                "type": "array",
                "items": {"type": "number"},
                "minItems": 4,
                "maxItems": 4,
                "description": "[y_min, x_min, y_max, x_max] in 0-1000 normalized coordinates"
            },
            "action_type": {
                "type": "string",
                "enum": ["click", "type", "scroll", "drag", "hover"],
                "description": "The type of action to perform on this element"
            },
            "action_params": {
                "type": "object",
                "description": "Additional parameters for the action (e.g., text to type, scroll amount)"
            },
            "confidence": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0,
                "description": "Confidence score for the detection"
            },
            "alternatives": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "label": {"type": "string"},
                        "bounding_box": {
                            "type": "array",
                            "items": {"type": "number"},
                            "minItems": 4,
                            "maxItems": 4
                        },
                        "confidence": {"type": "number"}
                    }
                },
                "description": "Alternative elements if multiple matches found"
            }
        },
        "required": ["element_found"]
    })
}

/// Build a Gemini request for element detection
pub fn build_element_detection_request(
    instruction: &str,
    screenshot_base64: &str,
    _model: &str,
) -> GeminiRequest {
    let prompt = format!(
        r#"Analyze this desktop application screenshot and locate the UI element described by: "{}"

Return a JSON response with:
1. "element_found": boolean - true if the element was found
2. "label": descriptive name of the element
3. "bounding_box": [y_min, x_min, y_max, x_max] in 0-1000 normalized coordinates
4. "action_type": one of ["click", "type", "scroll", "drag", "hover"]
5. "action_params": additional parameters (e.g., {{"text": "hello"}} for type action)
6. "confidence": float 0-1 indicating detection confidence
7. "alternatives": array of similar elements if ambiguous

Guidelines:
- For "click" actions, return the bounding box of the clickable element (button, link, icon, etc.)
- For "type" actions, identify the input field and extract any text to type from the instruction
- If multiple matches exist, return the most likely in the main result and others in "alternatives"
- Consider context: buttons, text fields, menus, icons, tabs, etc.
- Return element_found=false if the element cannot be located"#,
        instruction
    );

    GeminiRequest {
        contents: vec![Content {
            parts: vec![
                Part::Text { text: prompt },
                Part::InlineData {
                    inline_data: InlineData {
                        mime_type: "image/png".to_string(),
                        data: screenshot_base64.to_string(),
                    },
                },
            ],
        }],
        generation_config: GenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_schema: element_detection_schema(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_detection_schema() {
        let schema = element_detection_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["element_found"].is_object());
        assert!(schema["properties"]["bounding_box"].is_object());
    }

    #[test]
    fn test_build_request() {
        let request = build_element_detection_request(
            "click the save button",
            "base64data",
            "gemini-3-flash-preview",
        );
        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.contents[0].parts.len(), 2);
    }

    #[test]
    fn test_deserialize_element_detection_result() {
        let json = r#"{
            "element_found": true,
            "label": "Save Button",
            "bounding_box": [100, 200, 150, 300],
            "action_type": "click",
            "confidence": 0.95,
            "alternatives": []
        }"#;

        let result: ElementDetectionResult = serde_json::from_str(json).unwrap();
        assert!(result.element_found);
        assert_eq!(result.label, "Save Button");
        assert_eq!(result.bounding_box.len(), 4);
        assert_eq!(result.action_type, "click");
    }
}
