use serde_json::Value;
use crate::db::Database;
use crate::sse::SseBroadcaster;

pub struct ToolContext {
    pub db: Database,
    pub broadcaster: SseBroadcaster,
}

pub async fn handle_tool_call(
    name: &str,
    arguments: Value,
    ctx: &ToolContext,
) -> Result<Value, String> {
    match name {
        "register_agent" => register_agent(arguments, ctx).await,
        "get_next_task" => get_next_task(arguments, ctx).await,
        "update_task_status" => update_task_status(arguments, ctx).await,
        "add_task_comment" => add_task_comment(arguments, ctx).await,
        "create_task" => create_task(arguments, ctx).await,
        "list_tasks" => list_tasks(arguments, ctx).await,
        "get_task" => get_task(arguments, ctx).await,
        "set_output_path" => set_output_path(arguments, ctx).await,
        "heartbeat" => heartbeat(arguments, ctx).await,
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

async fn register_agent(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let role = args["role"].as_str().ok_or("Missing role")?;
    let agent = ctx.db.register_agent(agent_id, role).await.map_err(|e| e.to_string())?;
    ctx.broadcaster.broadcast(serde_json::json!({"event":"agent_registered","data":agent}).to_string());
    Ok(serde_json::json!({
        "message": format!("Agent '{}' registered with role '{}'", agent_id, role),
        "agent": agent
    }))
}

async fn get_next_task(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let role = args["role"].as_str().ok_or("Missing role")?;
    match ctx.db.get_next_task_for_role(agent_id, role).await.map_err(|e| e.to_string())? {
        None => Ok(serde_json::json!({"message": format!("No tasks available for role '{}'", role), "task": null})),
        Some(task) => {
            ctx.broadcaster.broadcast(serde_json::json!({"event":"task_updated","data":task}).to_string());
            Ok(serde_json::json!({"message":"Task assigned","task":task}))
        }
    }
}

async fn update_task_status(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let task_id = args["task_id"].as_str().ok_or("Missing task_id")?;
    let status = args["status"].as_str().ok_or("Missing status")?;
    let note = args["note"].as_str();

    let task = ctx.db.update_task(task_id, None, None, Some(status), None, None, None, None, None)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let agents = ctx.db.list_agents().await.map_err(|e| e.to_string())?;
    let agent_role = agents.iter().find(|a| a.agent_id == agent_id).map(|a| a.role.as_str());
    let detail = note
        .map(|n| format!("Status → '{}': {}", status, n))
        .unwrap_or_else(|| format!("Status → '{}'", status));

    let entry = ctx.db.add_activity(task_id, Some(agent_id), agent_role, "status_changed", Some(&detail))
        .await.map_err(|e| e.to_string())?;

    ctx.broadcaster.broadcast(serde_json::json!({"event":"task_updated","data":task}).to_string());
    ctx.broadcaster.broadcast(serde_json::json!({"event":"activity_added","data":entry}).to_string());
    Ok(serde_json::json!({"task":task}))
}

async fn add_task_comment(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let task_id = args["task_id"].as_str().ok_or("Missing task_id")?;
    let comment = args["comment"].as_str().ok_or("Missing comment")?;

    let agents = ctx.db.list_agents().await.map_err(|e| e.to_string())?;
    let agent_role = agents.iter().find(|a| a.agent_id == agent_id).map(|a| a.role.as_str());

    let entry = ctx.db.add_activity(task_id, Some(agent_id), agent_role, "comment", Some(comment))
        .await.map_err(|e| e.to_string())?;

    ctx.broadcaster.broadcast(serde_json::json!({"event":"activity_added","data":entry}).to_string());
    Ok(serde_json::json!({"message":"Comment added","activity_id":entry.id}))
}

async fn create_task(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let title = args["title"].as_str().ok_or("Missing title")?;
    let description = args["description"].as_str();
    let priority = args["priority"].as_str().unwrap_or("medium");
    let assigned_role = args["assigned_role"].as_str();
    let tags: Vec<String> = args["tags"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let task = ctx.db.create_task(Some(agent_id), title, description, priority, assigned_role, &tags)
        .await.map_err(|e| e.to_string())?;

    ctx.broadcaster.broadcast(serde_json::json!({"event":"task_created","data":task}).to_string());
    Ok(serde_json::json!({"task":task}))
}

async fn list_tasks(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let status = args["status"].as_str().map(|s| s.to_string());
    let assigned_role = args["assigned_role"].as_str().map(|s| s.to_string());
    let assigned_agent_id = args["assigned_agent_id"].as_str().map(|s| s.to_string());
    let tasks = ctx.db.list_tasks(status, assigned_role, assigned_agent_id)
        .await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"tasks":tasks}))
}

async fn get_task(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let task_id = args["task_id"].as_str().ok_or("Missing task_id")?;
    match ctx.db.get_task_with_activity(task_id).await.map_err(|e| e.to_string())? {
        None => Err(format!("Task {} not found", task_id)),
        Some(task) => Ok(serde_json::json!({"task":task})),
    }
}

async fn set_output_path(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let task_id = args["task_id"].as_str().ok_or("Missing task_id")?;
    let output_path = args["output_path"].as_str().ok_or("Missing output_path")?;

    let task = ctx.db.update_task(task_id, None, None, None, None, None, None, Some(output_path), None)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let agents = ctx.db.list_agents().await.map_err(|e| e.to_string())?;
    let agent_role = agents.iter().find(|a| a.agent_id == agent_id).map(|a| a.role.as_str());

    ctx.db.add_activity(task_id, Some(agent_id), agent_role, "output_set", Some(&format!("Output: {}", output_path)))
        .await.map_err(|e| e.to_string())?;

    ctx.broadcaster.broadcast(serde_json::json!({"event":"task_updated","data":task}).to_string());
    Ok(serde_json::json!({"task":task}))
}

async fn heartbeat(args: Value, ctx: &ToolContext) -> Result<Value, String> {
    let agent_id = args["agent_id"].as_str().ok_or("Missing agent_id")?;
    let ts = ctx.db.heartbeat(agent_id).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"message":"Heartbeat received","agent_id":agent_id,"timestamp":ts}))
}

pub fn tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "register_agent",
            "description": "Register an agent so it appears in the system.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string", "description": "Unique identifier for this agent instance"},
                    "role": {"type": "string", "enum": ["coder", "reviewer", "tester", "docs_writer"]}
                },
                "required": ["agent_id", "role"]
            }
        },
        {
            "name": "get_next_task",
            "description": "Get the next available task for the agent's role and claim it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "role": {"type": "string", "enum": ["coder", "reviewer", "tester", "docs_writer"]}
                },
                "required": ["agent_id", "role"]
            }
        },
        {
            "name": "update_task_status",
            "description": "Move a task to a new status and log the transition.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "task_id": {"type": "string"},
                    "status": {"type": "string", "enum": ["backlog","in_progress","in_review","testing","docs_needed","done","blocked"]},
                    "note": {"type": "string"}
                },
                "required": ["agent_id", "task_id", "status"]
            }
        },
        {
            "name": "add_task_comment",
            "description": "Add a comment to a task's activity log without changing status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "task_id": {"type": "string"},
                    "comment": {"type": "string"}
                },
                "required": ["agent_id", "task_id", "comment"]
            }
        },
        {
            "name": "create_task",
            "description": "Create a new task (agents can spawn subtasks).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "priority": {"type": "string", "enum": ["low","medium","high","critical"]},
                    "assigned_role": {"type": "string", "enum": ["coder","reviewer","tester","docs_writer"]},
                    "tags": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["agent_id", "title"]
            }
        },
        {
            "name": "list_tasks",
            "description": "Query tasks with optional filters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "assigned_role": {"type": "string"},
                    "assigned_agent_id": {"type": "string"}
                }
            }
        },
        {
            "name": "get_task",
            "description": "Get full details of a task including its activity log.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {"type": "string"}
                },
                "required": ["task_id"]
            }
        },
        {
            "name": "set_output_path",
            "description": "Record the file path of an artifact produced for a task.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "task_id": {"type": "string"},
                    "output_path": {"type": "string"}
                },
                "required": ["agent_id", "task_id", "output_path"]
            }
        },
        {
            "name": "heartbeat",
            "description": "Agent sends a heartbeat to show it's alive.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"}
                },
                "required": ["agent_id"]
            }
        }
    ])
}
