use std::panic::Location;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmContextType {
    SchemaDoc,
    QueryPattern,
    BusinessRule,
    Example,
    Instruction,
}

impl LlmContextType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SchemaDoc => "schema_doc",
            Self::QueryPattern => "query_pattern",
            Self::BusinessRule => "business_rule",
            Self::Example => "example",
            Self::Instruction => "instruction",
        }
    }

    #[track_caller]
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "schema_doc" => Ok(Self::SchemaDoc),
            "query_pattern" => Ok(Self::QueryPattern),
            "business_rule" => Ok(Self::BusinessRule),
            "example" => Ok(Self::Example),
            "instruction" => Ok(Self::Instruction),
            _ => Err(crate::CoreError::Validation {
                message: format!("Invalid context type: {}", s),
                location: crate::ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
