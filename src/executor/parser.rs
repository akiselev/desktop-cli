/// Instruction parser for validating and preprocessing natural language instructions
/// Since we're using Gemini for interpretation, this mainly does validation and cleanup

use crate::error::{DesktopMcpError, Result};

/// Parse and validate an instruction
pub fn parse_instruction(instruction: &str) -> Result<String> {
    let trimmed = instruction.trim();

    if trimmed.is_empty() {
        return Err(DesktopMcpError::ConfigError(
            "Instruction cannot be empty".to_string(),
        ));
    }

    if trimmed.len() > 500 {
        return Err(DesktopMcpError::ConfigError(
            "Instruction too long (max 500 characters)".to_string(),
        ));
    }

    Ok(trimmed.to_string())
}

/// Validate a list of instructions
pub fn validate_instructions(instructions: &[String]) -> Result<Vec<String>> {
    if instructions.is_empty() {
        return Err(DesktopMcpError::ConfigError(
            "Instruction list cannot be empty".to_string(),
        ));
    }

    if instructions.len() > 50 {
        return Err(DesktopMcpError::ConfigError(
            "Too many instructions (max 50)".to_string(),
        ));
    }

    let parsed: Result<Vec<String>> = instructions
        .iter()
        .enumerate()
        .map(|(i, inst)| {
            parse_instruction(inst).map_err(|e| {
                DesktopMcpError::ConfigError(format!("Instruction {}: {}", i + 1, e))
            })
        })
        .collect();

    parsed
}

/// Detect if an instruction is potentially dangerous
/// This is a simple heuristic - not comprehensive security
pub fn is_dangerous_instruction(instruction: &str) -> bool {
    let lower = instruction.to_lowercase();
    let dangerous_keywords = [
        "delete",
        "format",
        "shutdown",
        "reboot",
        "rm -rf",
        "del /f",
        "erase",
        "wipe",
        "destroy",
    ];

    dangerous_keywords
        .iter()
        .any(|keyword| lower.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instruction_valid() {
        let result = parse_instruction("  click the save button  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "click the save button");
    }

    #[test]
    fn test_parse_instruction_empty() {
        let result = parse_instruction("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_instruction_too_long() {
        let long_instruction = "a".repeat(501);
        let result = parse_instruction(&long_instruction);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_instructions() {
        let instructions = vec![
            "click button".to_string(),
            "type hello".to_string(),
            "press enter".to_string(),
        ];
        let result = validate_instructions(&instructions);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_validate_instructions_empty() {
        let instructions: Vec<String> = vec![];
        let result = validate_instructions(&instructions);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_instructions_too_many() {
        let instructions = vec!["step".to_string(); 51];
        let result = validate_instructions(&instructions);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_dangerous_instruction() {
        assert!(is_dangerous_instruction("delete all files"));
        assert!(is_dangerous_instruction("format the drive"));
        assert!(is_dangerous_instruction("shutdown the computer"));
        assert!(!is_dangerous_instruction("click the save button"));
        assert!(!is_dangerous_instruction("type hello world"));
    }
}
