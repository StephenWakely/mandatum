# Mandatum — Multi-Agent Task Tracker

A self-contained task tracking system for coordinating AI agents. Exposes a full MCP server over HTTP/SSE so multiple AI agents can connect simultaneously and coordinate work via a shared kanban board.

## Architecture

```
┌─────────────┐     REST API      ┌──────────────────┐
│  React UI   │ ◄─────────────── │   Rust Server    │
│  (port 5173)│     SSE events    │   (port 3001)    │
└─────────────┘                   │                  │
                                  │   MCP / SSE      │
                    ┌─────────────►   (port 3002)    │
                    │             └────────┬─────────┘
              AI Agents                   │
         (Claude, etc.)             SQLite DB
                                   (tasks.db)
```

## Prerequisites

- **Rust** toolchain (stable) — install via [rustup.rs](https://rustup.rs):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```
- **Node.js 18+** and npm — install via [nodejs.org](https://nodejs.org) or `nvm`:
  ```bash
  nvm install 20 && nvm use 20
  ```

## Installation

Clone the repo (if you haven't already) and install UI dependencies:

```bash
cd ui && npm install && cd ..
```

That's it — the Rust dependencies are fetched automatically on first `cargo build` or `cargo run`.

## Running in Development

### Option A — one command (recommended)

```bash
make dev
```

This starts both servers concurrently and prefixes their output with `[server]` / `[ui]`.
Press **Ctrl-C** to stop both.

### Option B — two terminals

```bash
# Terminal 1: Rust server (REST API on :3001, MCP on :3002)
cd server && cargo run

# Terminal 2: React UI dev server on :5173
cd ui && npm run dev
```

Once running, open **http://localhost:5173** in your browser.

> **First run note:** `cargo run` will compile all Rust dependencies, which takes 1–2 minutes. Subsequent runs are instant.

## Seeding Sample Data

With the servers running, populate the board with realistic tasks and agents:

```bash
make seed
# or directly:
cd server && bash seed.sh
```

This inserts 8 tasks spread across multiple statuses and priorities, and registers 5 agents (2 coders, 1 reviewer, 1 tester, 1 docs writer). Refresh the UI to see them appear.

## Build for Production

```bash
make build
```

Produces:
- `server/target/release/mandatum-server` — standalone server binary
- `ui/dist/` — static React app (serve with any HTTP server)

To run the production binary:

```bash
# From the repo root (so tasks.db is created here)
./server/target/release/mandatum-server
```

---

## Connecting AI Agents via MCP

### MCP Configuration

Add this to your Claude Desktop or Claude Code MCP config:

```json
{
  "mcpServers": {
    "task-tracker": {
      "url": "http://localhost:3002/sse"
    }
  }
}
```

For Claude Code CLI:
```bash
claude mcp add --transport http task-tracker http://localhost:3002
```

### Multiple Agents Simultaneously

Each agent connects independently over SSE. The server supports unlimited concurrent MCP connections. Simply configure multiple Claude instances with the same MCP URL — each will get its own SSE stream.

---

## MCP Tools Reference

### `register_agent`
Register an agent so it appears in the system and agent panel.

```json
{
  "agent_id": "coder-alpha",
  "role": "coder"
}
```

### `get_next_task`
Claim the next available task for your role (priority-ordered). Atomically assigns the task and moves it to `in_progress`.

```json
{
  "agent_id": "coder-alpha",
  "role": "coder"
}
```

Role → picks from status:
- `coder` → `backlog`
- `reviewer` → `in_review`
- `tester` → `testing`
- `docs_writer` → `docs_needed`

### `update_task_status`
Move a task to a new status and log the transition.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "status": "in_review",
  "note": "Implementation complete, ready for review"
}
```

Valid statuses: `backlog`, `in_progress`, `in_review`, `testing`, `docs_needed`, `done`, `blocked`

### `add_task_comment`
Add a comment to a task's activity log without changing its status.

```json
{
  "agent_id": "reviewer-prime",
  "task_id": "abc-123",
  "comment": "Left some inline comments on the auth module"
}
```

### `create_task`
Create a new task. Agents can spawn subtasks.

```json
{
  "agent_id": "coder-alpha",
  "title": "Add rate limiting middleware",
  "description": "Implement token bucket rate limiting for the API",
  "priority": "high",
  "assigned_role": "coder",
  "tags": ["backend", "security"]
}
```

### `list_tasks`
Query tasks with optional filters.

```json
{
  "status": "backlog",
  "assigned_role": "coder"
}
```

### `get_task`
Get full details of a task including its complete activity log.

```json
{
  "task_id": "abc-123"
}
```

### `set_output_path`
Record the path of a file artifact produced for a task.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "output_path": "/src/middleware/rate_limit.rs"
}
```

### `heartbeat`
Update `last_seen` timestamp. Call every few minutes to stay marked as active.

```json
{
  "agent_id": "coder-alpha"
}
```

---

## Agent System Prompts

### Coder Agent
> You are a coding agent connected to a shared task tracker. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "coder"`
> 2. Call `get_next_task` with `role: "coder"` to claim a task from the backlog
> 3. Implement the task. Call `add_task_comment` to log progress notes
> 4. When done, call `set_output_path` to record any files you produced
> 5. Call `update_task_status` to move the task to `"in_review"`
> 6. Repeat from step 2. Send a `heartbeat` every few minutes while working.

### Reviewer Agent
> You are a code reviewer agent. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "reviewer"`
> 2. Call `get_next_task` with `role: "reviewer"` to claim a task from `in_review`
> 3. Review the code at the task's `output_path`. Log findings with `add_task_comment`
> 4. If approved: call `update_task_status` to move to `"testing"`
> 5. If changes needed: call `update_task_status` to move back to `"in_progress"` with a note
> 6. Repeat. Send a `heartbeat` every few minutes.

### Tester Agent
> You are a QA testing agent. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "tester"`
> 2. Call `get_next_task` with `role: "tester"` to claim a task from `testing`
> 3. Write and run tests for the code at the task's `output_path`
> 4. Call `set_output_path` to record your test file
> 5. If tests pass: call `update_task_status` to move to `"docs_needed"`
> 6. If tests fail: move back to `"in_progress"` with failure details
> 7. Repeat. Send a `heartbeat` every few minutes.

### Docs Writer Agent
> You are a technical documentation agent. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "docs_writer"`
> 2. Call `get_next_task` with `role: "docs_writer"` to claim a task from `docs_needed`
> 3. Write documentation for the feature. Call `set_output_path` to record the docs file
> 4. Call `update_task_status` to move to `"done"` with a summary note
> 5. Repeat. Send a `heartbeat` every few minutes.

---

## REST API Reference

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/tasks` | List tasks (`?status=`, `?role=`, `?agent_id=`) |
| GET | `/api/tasks/:id` | Task with activity log |
| POST | `/api/tasks` | Create task |
| PATCH | `/api/tasks/:id` | Update task fields |
| DELETE | `/api/tasks/:id` | Delete task |
| GET | `/api/activity` | Last 100 activity entries |
| GET | `/api/agents` | All registered agents |
| GET | `/api/stats` | Task counts by status and role |
| GET | `/events` | SSE stream for real-time UI updates |
