use crate::{CoreError, CoreResult};

use std::panic::Location;
use std::str::FromStr;

use error_location::ErrorLocation;
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
}

impl FromStr for LlmContextType {
    type Err = CoreError;

    #[track_caller]
    fn from_str(s: &str) -> CoreResult<Self> {
        match s {
            "schema_doc" => Ok(Self::SchemaDoc),
            "query_pattern" => Ok(Self::QueryPattern),
            "business_rule" => Ok(Self::BusinessRule),
            "example" => Ok(Self::Example),
            "instruction" => Ok(Self::Instruction),
            _ => Err(CoreError::Validation {
                message: format!("Invalid context type: {}", s),
                field: Some("instruction".into()),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
