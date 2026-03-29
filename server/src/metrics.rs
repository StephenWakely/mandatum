use dogstatsd_rs::{Client, ClientBuilder, Tag};
use tracing::warn;

/// Thin wrapper around the DogStatsD client.
/// All methods are fire-and-forget — errors are logged but never propagate.
/// If the client failed to initialise (no agent configured) every call is a no-op.
pub struct Metrics {
    client: Option<Client>,
}

impl Metrics {
    /// Build a client. Returns a no-op instance on failure so the server
    /// always starts regardless of whether a DogStatsD agent is reachable.
    pub fn new() -> Self {
        let result = ClientBuilder::new("")   // falls back to DD_DOGSTATSD_URL / DD_AGENT_HOST
            .namespace("mandatum")
            .global_tags([
                Tag::new("project:mandatum").expect("valid tag"),
            ])
            .build();

        match result {
            Ok(client) => {
                Self { client: Some(client) }
            }
            Err(e) => {
                warn!(error = %e, "DogStatsD client failed to initialise — metrics disabled");
                Self { client: None }
            }
        }
    }

    // ── Tasks ─────────────────────────────────────────────────────────────────

    /// Increment when any task is created.
    pub fn task_created(&self) {
        self.count("task.created", 1, &[]);
        self.event("Task created", "A new task was added to the board");
    }

    /// Increment when a task's status changes. Tags with from/to column names.
    pub fn task_status_changed(&self, from: &str, to: &str) {
        self.count(
            "task.status_changed",
            1,
            &[
                &format!("from_status:{}", from),
                &format!("to_status:{}", to),
            ],
        );
    }

    /// Increment when a task moves to Done. Also records the duration since creation.
    pub fn task_done(&self, title: &str, created_at: &str) {
        self.count("task.done", 1, &[]);
        self.event("Task done", &format!("Task completed: {}", title));

        // Duration from creation to now, in seconds, as a distribution.
        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_at) {
            let duration = chrono::Utc::now()
                .signed_duration_since(created.with_timezone(&chrono::Utc))
                .num_seconds();
            if duration >= 0 {
                self.distribution("task.duration_seconds", duration as f64, &[]);
            }
        }
    }

    // ── Commits ───────────────────────────────────────────────────────────────

    /// Increment when an agent records a commit.
    pub fn commit_created(&self) {
        self.count("commit.created", 1, &[]);
    }

    // ── Agents ────────────────────────────────────────────────────────────────

    /// Increment when an agent registers or heartbeats.
    pub fn agent_heartbeat(&self, role: &str) {
        self.count("agent.heartbeat", 1, &[&format!("role:{}", role)]);
    }

    /// Increment when an agent claims a task.
    pub fn task_claimed(&self, role: &str) {
        self.count("task.claimed", 1, &[&format!("role:{}", role)]);
    }

    /// Increment when an agent polls but the queue is empty for its role.
    pub fn task_poll_empty(&self, role: &str) {
        self.count("task.poll_empty", 1, &[&format!("role:{}", role)]);
    }

    /// Increment when a reviewer requests changes.
    pub fn review_changes_requested(&self) {
        self.count("review.changes_requested", 1, &[]);
    }

    /// Increment when a reviewer approves.
    pub fn review_approved(&self) {
        self.count("review.approved", 1, &[]);
    }

    /// Increment when a task is blocked after too many review cycles.
    pub fn task_blocked(&self) {
        self.count("task.blocked", 1, &[]);
    }

    /// Distribution of time (seconds) from when an agent claimed a task to when it was completed.
    pub fn task_claim_duration_seconds(&self, role: &str, seconds: f64) {
        self.distribution("task.claim_duration_seconds", seconds, &[&format!("role:{}", role)]);
    }

    /// Gauge the current size of each queue (one call per status).
    pub fn queue_sizes(&self, counts: &std::collections::HashMap<String, i64>) {
        let Some(ref c) = self.client else { return };
        for (status, count) in counts {
            let tag_str = format!("queue:{}", status);
            let mut builder = c.gauge("queue.size").value(*count as f64);
            if let Ok(tag) = dogstatsd_rs::Tag::sanitise(&tag_str) {
                builder = builder.tag(tag);
            }
            if let Err(e) = builder.send() {
                warn!(metric = "queue.size", error = %e, "metric send failed");
            }
        }
    }

    // ── Internals ─────────────────────────────────────────────────────────────

    fn count(&self, name: &str, value: i64, tags: &[&str]) {
        let Some(ref c) = self.client else { return };
        let mut builder = c.count(name).value(value);
        for t in tags {
            if let Ok(tag) = Tag::sanitise(*t) {
                builder = builder.tag(tag);
            }
        }
        if let Err(e) = builder.send() {
            warn!(metric = name, error = %e, "metric send failed");
        }
    }

    fn distribution(&self, name: &str, value: f64, tags: &[&str]) {
        let Some(ref c) = self.client else { return };
        let mut builder = c.distribution(name).value(value);
        for t in tags {
            if let Ok(tag) = Tag::sanitise(*t) {
                builder = builder.tag(tag);
            }
        }
        if let Err(e) = builder.send() {
            warn!(metric = name, error = %e, "metric send failed");
        }
    }

    fn event(&self, title: &str, text: &str) {
        let Some(ref c) = self.client else { return };
        if let Err(e) = c.event(title, text).send() {
            warn!(event = title, error = %e, "event send failed");
        }
    }
}
