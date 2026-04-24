use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_MAX_CONCURRENT: usize = 5;

#[derive(Deserialize, Clone, Debug)]
pub struct AgentRoleConfig {
    #[serde(rename = "type", default = "default_agent_type")]
    pub agent_type: String,
    pub additional_instructions: Option<String>,
    pub max_concurrent: Option<usize>,
    pub caveman: Option<bool>,
}

fn default_agent_type() -> String {
    "claude".to_string()
}

fn default_agents_dir() -> String {
    "agents".to_string()
}

fn default_max_concurrent() -> usize {
    DEFAULT_MAX_CONCURRENT
}

fn default_caveman() -> bool {
    true
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MandatumConfig {
    pub project_dir: Option<String>,
    #[serde(default = "default_agents_dir")]
    pub agents_dir: String,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_caveman")]
    pub caveman: bool,
    #[serde(default)]
    pub agents: HashMap<String, AgentRoleConfig>,
}

impl MandatumConfig {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn agent_type(&self, role: &str) -> &str {
        self.agents
            .get(role)
            .map(|a| a.agent_type.as_str())
            .unwrap_or("claude")
    }

    pub fn additional_instructions(&self, role: &str) -> &str {
        self.agents
            .get(role)
            .and_then(|a| a.additional_instructions.as_deref())
            .unwrap_or("")
    }

    pub fn max_concurrent_for_role(&self, role: &str) -> usize {
        self.agents
            .get(role)
            .and_then(|a| a.max_concurrent)
            .unwrap_or(self.max_concurrent)
    }

    pub fn caveman_for_role(&self, role: &str) -> bool {
        self.agents
            .get(role)
            .and_then(|a| a.caveman)
            .unwrap_or(self.caveman)
    }
}
