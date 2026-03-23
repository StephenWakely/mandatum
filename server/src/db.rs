use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_rusqlite::Connection;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub assigned_role: Option<String>,
    pub assigned_agent_id: Option<String>,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    pub output_path: Option<String>,
    pub tags: Vec<String>,
    // git fields
    pub branch_name: Option<String>,
    pub base_branch: String,
    pub latest_commit: Option<String>,
    pub commit_count: i64,
    pub pr_url: Option<String>,
    pub worktree_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithActivity {
    #[serde(flatten)]
    pub task: Task,
    pub activity: Vec<ActivityEntry>,
    pub commits: Vec<Commit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub task_id: String,
    pub agent_id: Option<String>,
    pub hash: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    pub task_id: String,
    pub agent_id: Option<String>,
    pub agent_role: Option<String>,
    pub action: String,
    pub detail: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub agent_id: String,
    pub role: String,
    pub last_seen: Option<String>,
    pub current_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total: i64,
    pub by_status: std::collections::HashMap<String, i64>,
    pub by_role: std::collections::HashMap<String, i64>,
}

/// Slugify a task title for use in branch names.
pub fn branch_slug(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // collapse runs of dashes, strip leading/trailing
    let parts: Vec<&str> = slug.split('-').filter(|s| !s.is_empty()).collect();
    parts.join("-").chars().take(40).collect()
}

/// Suggest a branch name for a task.
pub fn suggest_branch(task_id: &str, title: &str) -> String {
    let short_id = &task_id[..task_id.len().min(8)];
    format!("feature/{}-{}", short_id, branch_slug(title))
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let tags_str: String = row.get::<_, String>(10).unwrap_or_else(|_| "[]".to_string());
    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
    Ok(Task {
        id:                 row.get(0)?,
        title:              row.get(1)?,
        description:        row.get(2)?,
        status:             row.get(3)?,
        assigned_role:      row.get(4)?,
        assigned_agent_id:  row.get(5)?,
        priority:           row.get(6)?,
        created_at:         row.get(7)?,
        updated_at:         row.get(8)?,
        output_path:        row.get(9)?,
        tags,
        branch_name:   row.get(11).unwrap_or(None),
        base_branch:   row.get::<_, Option<String>>(12).unwrap_or(None).unwrap_or_else(|| "main".to_string()),
        latest_commit: row.get(13).unwrap_or(None),
        commit_count:  row.get::<_, Option<i64>>(14).unwrap_or(None).unwrap_or(0),
        pr_url:        row.get(15).unwrap_or(None),
        worktree_path: row.get(16).unwrap_or(None),
    })
}

const TASK_SELECT: &str = "SELECT id, title, description, status, assigned_role, assigned_agent_id, \
    priority, created_at, updated_at, output_path, tags, \
    branch_name, base_branch, latest_commit, commit_count, pr_url, worktree_path FROM tasks";

#[derive(Clone)]
pub struct Database {
    conn: Arc<Connection>,
}

impl Database {
    pub async fn new(path: &str) -> Result<Self, tokio_rusqlite::Error> {
        let conn = Connection::open(path).await?;
        let db = Database { conn: Arc::new(conn) };
        db.init_schema().await?;
        Ok(db)
    }

    async fn init_schema(&self) -> Result<(), tokio_rusqlite::Error> {
        self.conn.call(|conn| {
            conn.execute_batch("
                PRAGMA journal_mode=WAL;
                CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT,
                    status TEXT NOT NULL DEFAULT 'backlog',
                    assigned_role TEXT,
                    assigned_agent_id TEXT,
                    priority TEXT NOT NULL DEFAULT 'medium',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    output_path TEXT,
                    tags TEXT DEFAULT '[]'
                );
                CREATE TABLE IF NOT EXISTS activity_log (
                    id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    agent_id TEXT,
                    agent_role TEXT,
                    action TEXT NOT NULL,
                    detail TEXT,
                    timestamp TEXT NOT NULL,
                    FOREIGN KEY (task_id) REFERENCES tasks(id)
                );
                CREATE TABLE IF NOT EXISTS agents (
                    agent_id TEXT PRIMARY KEY,
                    role TEXT NOT NULL,
                    last_seen TEXT,
                    current_task_id TEXT
                );
                CREATE TABLE IF NOT EXISTS commits (
                    id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    agent_id TEXT,
                    hash TEXT NOT NULL,
                    message TEXT NOT NULL,
                    timestamp TEXT NOT NULL,
                    FOREIGN KEY (task_id) REFERENCES tasks(id)
                );
            ")?;
            // Migrate existing DBs — ignore errors if columns already exist
            let _ = conn.execute("ALTER TABLE tasks ADD COLUMN branch_name TEXT", []);
            let _ = conn.execute("ALTER TABLE tasks ADD COLUMN base_branch TEXT DEFAULT 'main'", []);
            let _ = conn.execute("ALTER TABLE tasks ADD COLUMN latest_commit TEXT", []);
            let _ = conn.execute("ALTER TABLE tasks ADD COLUMN commit_count INTEGER DEFAULT 0", []);
            let _ = conn.execute("ALTER TABLE tasks ADD COLUMN pr_url TEXT", []);
            let _ = conn.execute("ALTER TABLE tasks ADD COLUMN worktree_path TEXT", []);
            Ok(())
        }).await
    }

    pub async fn list_tasks(
        &self,
        status: Option<String>,
        role: Option<String>,
        agent_id: Option<String>,
    ) -> Result<Vec<Task>, tokio_rusqlite::Error> {
        self.conn.call(move |conn| {
            let mut query = TASK_SELECT.to_string();
            let mut where_parts: Vec<String> = Vec::new();
            if let Some(ref s) = status {
                where_parts.push(format!("status = '{}'", s.replace('\'', "''")));
            }
            if let Some(ref r) = role {
                where_parts.push(format!("assigned_role = '{}'", r.replace('\'', "''")));
            }
            if let Some(ref a) = agent_id {
                where_parts.push(format!("assigned_agent_id = '{}'", a.replace('\'', "''")));
            }
            if !where_parts.is_empty() {
                query.push_str(" WHERE ");
                query.push_str(&where_parts.join(" AND "));
            }
            query.push_str(" ORDER BY created_at DESC");
            let mut stmt = conn.prepare(&query)?;
            let tasks = stmt.query_map([], row_to_task)?.collect::<Result<Vec<_>, _>>()?;
            Ok(tasks)
        }).await
    }

    pub async fn get_task(&self, id: &str) -> Result<Option<Task>, tokio_rusqlite::Error> {
        let id = id.to_string();
        self.conn.call(move |conn| {
            Ok(conn.query_row(
                &format!("{} WHERE id = ?1", TASK_SELECT),
                params![id],
                row_to_task,
            ).optional()?)
        }).await
    }

    pub async fn get_task_with_activity(&self, id: &str) -> Result<Option<TaskWithActivity>, tokio_rusqlite::Error> {
        let id_owned = id.to_string();
        let task = self.get_task(&id_owned).await?;
        match task {
            None => Ok(None),
            Some(task) => {
                let task_id = task.id.clone();
                let activity = self.get_activity_for_task(&task_id).await?;
                let commits = self.list_commits_for_task(&task_id).await?;
                Ok(Some(TaskWithActivity { task, activity, commits }))
            }
        }
    }

    pub async fn get_activity_for_task(&self, task_id: &str) -> Result<Vec<ActivityEntry>, tokio_rusqlite::Error> {
        let task_id = task_id.to_string();
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, agent_id, agent_role, action, detail, timestamp \
                 FROM activity_log WHERE task_id = ?1 ORDER BY timestamp DESC"
            )?;
            let entries = stmt.query_map(params![task_id], |row| Ok(ActivityEntry {
                id: row.get(0)?,
                task_id: row.get(1)?,
                agent_id: row.get(2)?,
                agent_role: row.get(3)?,
                action: row.get(4)?,
                detail: row.get(5)?,
                timestamp: row.get(6)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }).await
    }

    pub async fn list_commits_for_task(&self, task_id: &str) -> Result<Vec<Commit>, tokio_rusqlite::Error> {
        let task_id = task_id.to_string();
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, agent_id, hash, message, timestamp \
                 FROM commits WHERE task_id = ?1 ORDER BY timestamp DESC"
            )?;
            let commits = stmt.query_map(params![task_id], |row| Ok(Commit {
                id: row.get(0)?,
                task_id: row.get(1)?,
                agent_id: row.get(2)?,
                hash: row.get(3)?,
                message: row.get(4)?,
                timestamp: row.get(5)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            Ok(commits)
        }).await
    }

    pub async fn add_commit(
        &self,
        task_id: &str,
        agent_id: Option<&str>,
        hash: &str,
        message: &str,
    ) -> Result<Commit, tokio_rusqlite::Error> {
        let commit = Commit {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            hash: hash.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        let c = commit.clone();
        let task_id_owned = task_id.to_string();
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO commits (id, task_id, agent_id, hash, message, timestamp) VALUES (?1,?2,?3,?4,?5,?6)",
                params![c.id, c.task_id, c.agent_id, c.hash, c.message, c.timestamp],
            )?;
            // update task latest_commit and increment commit_count
            conn.execute(
                "UPDATE tasks SET latest_commit = ?1, commit_count = commit_count + 1, updated_at = ?2 WHERE id = ?3",
                params![c.hash, c.timestamp, task_id_owned],
            )?;
            Ok(())
        }).await?;
        Ok(commit)
    }

    pub async fn create_task(
        &self,
        agent_id: Option<&str>,
        title: &str,
        description: Option<&str>,
        priority: &str,
        assigned_role: Option<&str>,
        tags: &[String],
    ) -> Result<Task, tokio_rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());

        let task = Task {
            id: id.clone(),
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
            status: "backlog".to_string(),
            assigned_role: assigned_role.map(|s| s.to_string()),
            assigned_agent_id: None,
            priority: priority.to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            output_path: None,
            tags: tags.to_vec(),
            branch_name: None,
            base_branch: "main".to_string(),
            latest_commit: None,
            commit_count: 0,
            pr_url: None,
            worktree_path: None,
        };

        let t = task.clone();
        let agent_id_owned = agent_id.map(|s| s.to_string());

        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO tasks (id, title, description, status, assigned_role, assigned_agent_id, \
                 priority, created_at, updated_at, output_path, tags, branch_name, base_branch, \
                 latest_commit, commit_count, pr_url, worktree_path) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![
                    t.id, t.title, t.description, t.status, t.assigned_role, t.assigned_agent_id,
                    t.priority, t.created_at, t.updated_at, t.output_path, tags_json,
                    t.branch_name, t.base_branch, t.latest_commit, t.commit_count, t.pr_url,
                    t.worktree_path
                ],
            )?;
            let aid = Uuid::new_v4().to_string();
            let now2 = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO activity_log (id, task_id, agent_id, agent_role, action, detail, timestamp) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![aid, t.id, agent_id_owned, Option::<String>::None, "created", Option::<String>::None, now2],
            )?;
            Ok(())
        }).await?;

        Ok(task)
    }

    pub async fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        assigned_role: Option<&str>,
        assigned_agent_id: Option<&str>,
        output_path: Option<&str>,
        tags: Option<&[String]>,
        branch_name: Option<&str>,
        base_branch: Option<&str>,
        latest_commit: Option<&str>,
        pr_url: Option<&str>,
        worktree_path: Option<&str>,
    ) -> Result<Option<Task>, tokio_rusqlite::Error> {
        let id = id.to_string();
        let title = title.map(|s| s.to_string());
        let description = description.map(|s| s.to_string());
        let status = status.map(|s| s.to_string());
        let priority = priority.map(|s| s.to_string());
        let assigned_role = assigned_role.map(|s| s.to_string());
        // Sentinel: empty string means "clear the field to NULL"
        let clear_agent = assigned_agent_id.map_or(false, |s| s.is_empty());
        let assigned_agent_id = assigned_agent_id.filter(|s| !s.is_empty()).map(|s| s.to_string());
        let output_path = output_path.map(|s| s.to_string());
        let tags = tags.map(|t| t.to_vec());
        let branch_name = branch_name.map(|s| s.to_string());
        let base_branch = base_branch.map(|s| s.to_string());
        let latest_commit = latest_commit.map(|s| s.to_string());
        let pr_url = pr_url.map(|s| s.to_string());
        let worktree_path = worktree_path.map(|s| s.to_string());

        self.conn.call(move |conn| {
            let existing = conn.query_row(
                &format!("{} WHERE id = ?1", TASK_SELECT),
                params![id],
                row_to_task,
            ).optional()?;

            let existing = match existing {
                Some(t) => t,
                None => return Ok(None),
            };

            let new_title              = title.unwrap_or(existing.title);
            let new_description        = description.or(existing.description);
            let new_status             = status.unwrap_or(existing.status.clone());
            let new_priority           = priority.unwrap_or(existing.priority);
            // When status changes, derive the role and clear the agent so the
            // next agent of the right type can pick it up freely.
            let status_changed = new_status != existing.status;
            let derived_role: Option<String> = assigned_role.or_else(|| match new_status.as_str() {
                "in_review"   => Some("reviewer".to_string()),
                "testing"     => Some("tester".to_string()),
                "docs_needed" => Some("docs_writer".to_string()),
                _             => existing.assigned_role,
            });
            let new_assigned_role     = derived_role;
            let new_assigned_agent_id = if status_changed || clear_agent {
                assigned_agent_id  // None when clearing or status changed
            } else {
                assigned_agent_id.or(existing.assigned_agent_id)
            };
            let new_output_path        = output_path.or(existing.output_path);
            let new_tags               = tags.unwrap_or(existing.tags);
            let new_branch_name        = branch_name.or(existing.branch_name);
            let new_base_branch        = base_branch.unwrap_or(existing.base_branch);
            let new_latest_commit      = latest_commit.or(existing.latest_commit);
            let new_pr_url             = pr_url.or(existing.pr_url);
            let new_worktree_path      = worktree_path.or(existing.worktree_path);
            let tags_json              = serde_json::to_string(&new_tags).unwrap_or_else(|_| "[]".to_string());
            let now                    = Utc::now().to_rfc3339();

            conn.execute(
                "UPDATE tasks SET title=?2, description=?3, status=?4, priority=?5, \
                 assigned_role=?6, assigned_agent_id=?7, output_path=?8, tags=?9, \
                 branch_name=?10, base_branch=?11, latest_commit=?12, pr_url=?13, \
                 worktree_path=?14, updated_at=?1 WHERE id=?15",
                params![
                    now, new_title, new_description, new_status, new_priority,
                    new_assigned_role, new_assigned_agent_id, new_output_path, tags_json,
                    new_branch_name, new_base_branch, new_latest_commit, new_pr_url,
                    new_worktree_path, id
                ],
            )?;

            Ok(Some(Task {
                id,
                title: new_title,
                description: new_description,
                status: new_status,
                assigned_role: new_assigned_role,
                assigned_agent_id: new_assigned_agent_id,
                priority: new_priority,
                created_at: existing.created_at,
                updated_at: now,
                output_path: new_output_path,
                tags: new_tags,
                branch_name: new_branch_name,
                base_branch: new_base_branch,
                latest_commit: new_latest_commit,
                commit_count: existing.commit_count,
                pr_url: new_pr_url,
                worktree_path: new_worktree_path,
            }))
        }).await
    }

    pub async fn delete_task(&self, id: &str) -> Result<bool, tokio_rusqlite::Error> {
        let id = id.to_string();
        self.conn.call(move |conn| {
            let n = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
            Ok(n > 0)
        }).await
    }

    pub async fn list_activity(&self, limit: i64) -> Result<Vec<ActivityEntry>, tokio_rusqlite::Error> {
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, agent_id, agent_role, action, detail, timestamp \
                 FROM activity_log ORDER BY timestamp DESC LIMIT ?1"
            )?;
            let entries = stmt.query_map(params![limit], |row| Ok(ActivityEntry {
                id: row.get(0)?,
                task_id: row.get(1)?,
                agent_id: row.get(2)?,
                agent_role: row.get(3)?,
                action: row.get(4)?,
                detail: row.get(5)?,
                timestamp: row.get(6)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }).await
    }

    pub async fn add_activity(
        &self,
        task_id: &str,
        agent_id: Option<&str>,
        agent_role: Option<&str>,
        action: &str,
        detail: Option<&str>,
    ) -> Result<ActivityEntry, tokio_rusqlite::Error> {
        let entry = ActivityEntry {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            agent_role: agent_role.map(|s| s.to_string()),
            action: action.to_string(),
            detail: detail.map(|s| s.to_string()),
            timestamp: Utc::now().to_rfc3339(),
        };
        let e = entry.clone();
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO activity_log (id, task_id, agent_id, agent_role, action, detail, timestamp) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![e.id, e.task_id, e.agent_id, e.agent_role, e.action, e.detail, e.timestamp],
            )?;
            Ok(())
        }).await?;
        Ok(entry)
    }

    pub async fn register_agent(&self, agent_id: &str, role: &str) -> Result<Agent, tokio_rusqlite::Error> {
        let agent = Agent {
            agent_id: agent_id.to_string(),
            role: role.to_string(),
            last_seen: Some(Utc::now().to_rfc3339()),
            current_task_id: None,
        };
        let a = agent.clone();
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO agents (agent_id, role, last_seen, current_task_id) VALUES (?1,?2,?3,?4)",
                params![a.agent_id, a.role, a.last_seen, a.current_task_id],
            )?;
            Ok(())
        }).await?;
        Ok(agent)
    }

    pub async fn list_agents(&self) -> Result<Vec<Agent>, tokio_rusqlite::Error> {
        self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT agent_id, role, last_seen, current_task_id FROM agents ORDER BY last_seen DESC"
            )?;
            let agents = stmt.query_map([], |row| Ok(Agent {
                agent_id: row.get(0)?,
                role: row.get(1)?,
                last_seen: row.get(2)?,
                current_task_id: row.get(3)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            Ok(agents)
        }).await
    }

    pub async fn heartbeat(&self, agent_id: &str) -> Result<String, tokio_rusqlite::Error> {
        let agent_id = agent_id.to_string();
        let now = Utc::now().to_rfc3339();
        let now2 = now.clone();
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2",
                params![now2, agent_id],
            )?;
            Ok(())
        }).await?;
        Ok(now)
    }

    pub async fn get_next_task_for_role(
        &self,
        agent_id: &str,
        role: &str,
    ) -> Result<Option<Task>, tokio_rusqlite::Error> {
        let agent_id = agent_id.to_string();
        let role = role.to_string();

        let target_status = match role.as_str() {
            "coder"       => "backlog",
            "reviewer"    => "in_review",
            "tester"      => "testing",
            "docs_writer" => "docs_needed",
            _ => return Ok(None),
        }.to_string();

        self.conn.call(move |conn| {
            let task_id: Option<String> = conn.query_row(
                "SELECT id FROM tasks \
                 WHERE status = ?1 \
                 AND (assigned_agent_id IS NULL OR assigned_agent_id = '') \
                 AND (assigned_role IS NULL OR assigned_role = '' OR assigned_role = ?2) \
                 ORDER BY CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, \
                 created_at ASC LIMIT 1",
                params![target_status, role],
                |row| row.get(0),
            ).optional()?;

            match task_id {
                None => Ok(None),
                Some(tid) => {
                    let now = Utc::now().to_rfc3339();
                    conn.execute(
                        "UPDATE tasks SET assigned_agent_id = ?1, status = 'in_progress', updated_at = ?2 WHERE id = ?3",
                        params![agent_id, now, tid],
                    )?;
                    conn.execute(
                        "UPDATE agents SET current_task_id = ?1, last_seen = ?2 WHERE agent_id = ?3",
                        params![tid, now, agent_id],
                    )?;
                    let aid = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO activity_log (id, task_id, agent_id, agent_role, action, detail, timestamp) \
                         VALUES (?1,?2,?3,?4,?5,?6,?7)",
                        params![aid, tid, agent_id, role, "claimed", format!("Claimed by {}", agent_id), now],
                    )?;
                    let task = conn.query_row(
                        &format!("{} WHERE id = ?1", TASK_SELECT),
                        params![tid],
                        row_to_task,
                    ).optional()?;
                    Ok(task)
                }
            }
        }).await
    }

    pub async fn reap_stale_tasks(&self, timeout_minutes: i64) -> Result<Vec<String>, tokio_rusqlite::Error> {
        self.conn.call(move |conn| {
            let cutoff = format!("-{} minutes", timeout_minutes);
            let now = Utc::now().to_rfc3339();

            let mut stmt = conn.prepare(
                "SELECT id FROM tasks WHERE status = 'in_progress' \
                 AND assigned_agent_id IS NOT NULL AND assigned_agent_id != '' \
                 AND assigned_agent_id IN \
                 (SELECT agent_id FROM agents WHERE last_seen < datetime('now', ?1))"
            )?;
            let ids: Vec<String> = stmt.query_map(params![cutoff], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            if !ids.is_empty() {
                conn.execute(
                    "UPDATE tasks SET status = 'backlog', assigned_agent_id = NULL, updated_at = ?1 \
                     WHERE status = 'in_progress' \
                     AND assigned_agent_id IS NOT NULL AND assigned_agent_id != '' \
                     AND assigned_agent_id IN \
                     (SELECT agent_id FROM agents WHERE last_seen < datetime('now', ?2))",
                    params![now, cutoff],
                )?;
            }
            Ok(ids)
        }).await
    }

    pub async fn reset_task(&self, id: &str) -> Result<Option<Task>, tokio_rusqlite::Error> {
        let id = id.to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE tasks SET status = 'backlog', assigned_agent_id = NULL, updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
            Ok(conn.query_row(
                &format!("{} WHERE id = ?1", TASK_SELECT),
                params![id],
                row_to_task,
            ).optional()?)
        }).await
    }

    pub async fn get_stats(&self) -> Result<Stats, tokio_rusqlite::Error> {
        self.conn.call(|conn| {
            let total: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
            let mut by_status = std::collections::HashMap::new();
            let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM tasks GROUP BY status")?;
            for row in stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))? {
                let (s, c) = row?;
                by_status.insert(s, c);
            }
            let mut by_role = std::collections::HashMap::new();
            let mut stmt = conn.prepare(
                "SELECT assigned_role, COUNT(*) FROM tasks WHERE assigned_role IS NOT NULL GROUP BY assigned_role"
            )?;
            for row in stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))? {
                let (r, c) = row?;
                by_role.insert(r, c);
            }
            Ok(Stats { total, by_status, by_role })
        }).await
    }
}
