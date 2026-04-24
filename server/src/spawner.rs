use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

use crate::config::MandatumConfig;
use crate::AppState;

pub struct Spawner {
    config: Arc<MandatumConfig>,
    state: Arc<AppState>,
    running_per_role: Arc<Mutex<HashMap<String, usize>>>,
    log_dir: PathBuf,
}

impl Spawner {
    pub fn new(config: Arc<MandatumConfig>, state: Arc<AppState>, log_dir: PathBuf) -> Self {
        Self {
            config,
            state,
            running_per_role: Arc::new(Mutex::new(HashMap::new())),
            log_dir,
        }
    }

    pub async fn run(self: Arc<Self>) {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            self.check_and_spawn().await;
        }
    }

    async fn check_and_spawn(&self) {
        for role in ["coder", "reviewer", "tester", "docs_writer"] {
            let max = self.config.max_concurrent_for_role(role);
            let running = *self
                .running_per_role
                .lock()
                .unwrap()
                .get(role)
                .unwrap_or(&0);
            let slots = max.saturating_sub(running);
            if slots == 0 {
                continue;
            }
            match self.state.db.count_available_tasks_for_role(role).await {
                Ok(0) => {}
                Ok(available) => {
                    for _ in 0..slots.min(available) {
                        self.spawn_agent(role).await;
                    }
                }
                Err(e) => tracing::warn!("DB check for role {} failed: {}", role, e),
            }
        }
    }

    async fn spawn_agent(&self, role: &str) {
        let agent_type = self.config.agent_type(role).to_string();
        let short_id = &Uuid::new_v4().to_string()[..8].to_string();
        let agent_id = format!("{}-{}", role, short_id);

        let script_name = match role {
            "docs_writer" => "run-docs.sh".to_string(),
            r => format!("run-{}.sh", r),
        };
        let script_path = PathBuf::from(&self.config.agents_dir)
            .join(&agent_type)
            .join(&script_name);

        if !script_path.exists() {
            tracing::warn!(
                "Agent script not found: {} — cannot spawn {} agent",
                script_path.display(),
                role
            );
            return;
        }

        let project_dir = self
            .state
            .repo_path
            .clone()
            .or_else(|| self.config.project_dir.clone())
            .unwrap_or_else(|| ".".to_string());

        let base_instructions = self.config.additional_instructions(role);
        let additional = if self.config.caveman_for_role(role) {
            let caveman = "Respond terse like caveman. Drop articles, filler words, \
                pleasantries, hedging. Fragments OK. Short synonyms preferred. \
                Technical terms exact. Code blocks unchanged.";
            if base_instructions.is_empty() {
                caveman.to_string()
            } else {
                format!("{}\n\n{}", caveman, base_instructions)
            }
        } else {
            base_instructions.to_string()
        };

        let log_path = self.log_dir.join(format!("{}.log", agent_id));
        let log_file = match tokio::fs::File::create(&log_path).await {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(
                    "Cannot create log file {}: {}",
                    log_path.display(),
                    e
                );
                return;
            }
        };

        let mut cmd = Command::new("bash");
        cmd.arg(&script_path)
            .env("AGENT_ID", &agent_id)
            .env("PROJECT_DIR", &project_dir)
            .env("MANDATUM_ONCE", "1")
            .env("ADDITIONAL_INSTRUCTIONS", &additional)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to spawn agent {}: {}", agent_id, e);
                return;
            }
        };

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        {
            let mut map = self.running_per_role.lock().unwrap();
            *map.entry(role.to_string()).or_insert(0) += 1;
        }

        tracing::info!(
            "Spawned {} agent {} for role {}",
            agent_type,
            agent_id,
            role
        );

        let broadcaster = self.state.broadcaster.clone();
        let agent_id_clone = agent_id.clone();
        let role_counter = Arc::clone(&self.running_per_role);
        let role_str = role.to_string();

        // Channel merges stdout + stderr into a single ordered stream.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let tx_err = tx.clone();

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(line);
            }
        });

        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx_err.send(line);
            }
        });

        tokio::spawn(async move {
            let mut log_writer =
                tokio::io::BufWriter::new(log_file);

            while let Some(line) = rx.recv().await {
                let ts = Utc::now().to_rfc3339();
                let _ = log_writer.write_all(line.as_bytes()).await;
                let _ = log_writer.write_all(b"\n").await;
                let _ = log_writer.flush().await;
                broadcaster.broadcast(
                    serde_json::json!({
                        "event": "agent_log",
                        "data": {
                            "agent_id": agent_id_clone,
                            "line": line,
                            "ts": ts
                        }
                    })
                    .to_string(),
                );
            }

            let _ = child.wait().await;

            let mut map = role_counter.lock().unwrap();
            let count = map.entry(role_str).or_insert(0);
            if *count > 0 {
                *count -= 1;
            }
            tracing::info!("Agent {} exited", agent_id_clone);
        });
    }
}
