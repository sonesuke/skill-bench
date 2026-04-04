//! Check validation models

use serde::{Deserialize, Serialize};

/// Check step for validation
/// TOML: command = {command = "...", skill = "..."}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckStep {
    pub name: String,
    pub command: CheckData,
    #[serde(default)]
    pub deny: bool,
}

/// Check data - command type and arguments
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CheckData {
    pub command: String,
    #[serde(default)]
    pub skill: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub db: Option<String>,
    #[serde(default)]
    pub expected: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub deny: Option<bool>,
    #[serde(default)]
    pub copy_to_output: Option<bool>,
}
