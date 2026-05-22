use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_MAX_CONCURRENT: usize = 5;
const DEFAULT_RUNTIME: &str = "bash";
const DEFAULT_DOCKER_IMAGE: &str = "mandatum-agent:latest";

#[derive(Deserialize, Clone, Debug)]
pub struct AgentRoleConfig {
    #[serde(rename = "type", default = "default_agent_type")]
    pub agent_type: String,
    pub additional_instructions: Option<String>,
    pub max_concurrent: Option<usize>,
    pub caveman: Option<bool>,
    /// Claude model alias (`sonnet`, `opus`, `haiku`) or full name.
    pub model: Option<String>,
    /// Claude effort level (`low`, `medium`, `high`, `xhigh`, `max`).
    pub effort: Option<String>,
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

fn default_runtime() -> String {
    DEFAULT_RUNTIME.to_string()
}

fn default_docker_image() -> String {
    DEFAULT_DOCKER_IMAGE.to_string()
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
    #[serde(default = "default_runtime")]
    pub runtime: String,
    #[serde(default = "default_docker_image")]
    pub docker_image: String,
    /// Shell command whose stdout is forwarded to the agent as
    /// `ANTHROPIC_AUTH_TOKEN`. Run once per spawn so tokens are always fresh.
    pub auth_token_helper: Option<String>,
    /// Multi-line headers forwarded as `ANTHROPIC_CUSTOM_HEADERS`. Use YAML
    /// `|` block scalar for newlines (e.g. gateway routing headers).
    pub anthropic_custom_headers: Option<String>,
    /// Default claude model — overridden by per-role `model`.
    pub model: Option<String>,
    /// Default claude effort level — overridden by per-role `effort`.
    pub effort: Option<String>,
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

    pub fn model_for_role(&self, role: &str) -> Option<String> {
        self.agents
            .get(role)
            .and_then(|a| a.model.clone())
            .or_else(|| self.model.clone())
    }

    pub fn effort_for_role(&self, role: &str) -> Option<String> {
        self.agents
            .get(role)
            .and_then(|a| a.effort.clone())
            .or_else(|| self.effort.clone())
    }
}
