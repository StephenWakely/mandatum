# Mandatum — Multi-Agent Task Tracker

A self-contained task tracking system for coordinating AI agents. Exposes a full MCP server over HTTP/SSE so multiple AI agents can connect simultaneously and coordinate work via a shared kanban board.

## Architecture

```
┌─────────────┐     REST API      ┌──────────────────┐
│  React UI   │ ◄───────────────  │   Rust Server    │
│  (port 5173)│     SSE events    │   (port 3001)    │
└─────────────┘                   │                  │
                                  │   MCP / SSE      │
                    ┌─────────────►   (port 3002)    │
                    │             └────────┬─────────┘
              AI Agents                    │
         (Claude, etc.)                SQLite DB
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

In the **project repo** that agents will work on, add `.worktrees/` to `.gitignore` so agent working directories are not committed:

```bash
echo '.worktrees/' >> /path/to/your/project/.gitignore
```

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

## Running Autonomous Agents

Mandatum supports both Claude and Codex runner scripts. In both cases, agents run non-interactively in a shell loop, complete a task cycle, exit, then restart automatically instead of waiting for user input.
The runner shell now claims the task and prepares the worktree before launching the model so agents do not edit the target repo's main checkout by accident.

### Prerequisites

- Either:
  - `claude` CLI on your `PATH` — install via [Claude Code](https://claude.ai/claude-code)
  - `codex` CLI on your `PATH`
- The Mandatum server must be running (`make dev` or `cargo run`)
- The project the agents will work on must be a git repository

### Quickstart

```bash
# Claude runners
/path/to/mandatum/agents/claude/run-all.sh /path/to/your/project

# Codex runners
/path/to/mandatum/agents/codex/run-all.sh /path/to/your/project
```

Press **Ctrl-C** to stop all agents cleanly.

### Runner layout

```text
agents/
├── claude/   # Claude Code runner scripts
└── codex/    # Codex runner scripts
```

### Running individual agents

Each role has its own script. All accept an optional agent ID and project directory:

```bash
# Usage: run-<role>.sh [agent-id] [project-dir]
# PROJECT_DIR env var is also accepted.

# Claude
/path/to/mandatum/agents/claude/run-coder.sh    coder-alpha   /path/to/project
/path/to/mandatum/agents/claude/run-reviewer.sh reviewer-1    /path/to/project
/path/to/mandatum/agents/claude/run-tester.sh   tester-1      /path/to/project
/path/to/mandatum/agents/claude/run-docs.sh     docs-1        /path/to/project

# Codex
/path/to/mandatum/agents/codex/run-coder.sh    coder-alpha   /path/to/project
/path/to/mandatum/agents/codex/run-reviewer.sh reviewer-1    /path/to/project
/path/to/mandatum/agents/codex/run-tester.sh   tester-1      /path/to/project
/path/to/mandatum/agents/codex/run-docs.sh     docs-1        /path/to/project
```

Run multiple coders in parallel by opening separate terminals with different agent IDs:

```bash
# Terminal 1
agents/codex/run-coder.sh coder-alpha /path/to/project

# Terminal 2
agents/codex/run-coder.sh coder-beta /path/to/project
```

### Project directory vs Mandatum directory

The scripts separate two concerns:

| Path | Purpose |
|------|---------|
| Mandatum directory | Where the tracker server and agent runner scripts live (`make dev`, `tasks.db`) |
| Project directory | The git repo agents check out branches and commit code into |

Claude runner config is resolved relative to `agents/claude/`. Codex runners use your normal `HOME` by default and register the `task-tracker` MCP server there automatically. Set `CODEX_RUN_HOME=/some/path` if you want an isolated Codex home for Mandatum-only runs. In both cases, the agent process runs with its cwd set to your project directory so all git operations land in the right repo.

### How the loop works

Claude scripts run `claude --print "<system-prompt>"` in a shell loop. Codex scripts run `codex exec "<system-prompt>"` in the same pattern. Before each model invocation, the wrapper claims the next task over MCP and creates or reuses the correct worktree in bash.

```
<agent command> "..." → agent runs, completes task, exits
        ↓ (10s pause)
<agent command> "..." → agent picks up next task
        ↓
        ...
```

If an agent's tokens are exhausted mid-task, the process exits and the loop restarts after 10 seconds. The server's watchdog will reap the stuck task back to the queue implied by its current `assigned_role` within 10 minutes so the right agent type can claim it again.

### Stale agent detection

The server automatically resets `in_progress` tasks whose assigned agent hasn't sent a heartbeat in **10 minutes**, moving them back to `backlog`. You can also trigger this manually:

```bash
# Reap all stale tasks at once
curl -X POST http://localhost:3001/api/tasks/reap

# Reset a specific task
curl -X POST http://localhost:3001/api/tasks/<task-id>/reset
```

The UI shows a ⚠ warning on any task card whose agent is stale, and a **Reset** button inside the task modal.

---

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
- `ui/dist/` — static React app

### Single-process deployment (recommended)

The server serves the React app directly — no separate web server needed:

```bash
# Build everything and start as a single process
make serve

# Or manually:
./server/target/release/mandatum-server --ui ui/dist
```

Open **http://localhost:3001** — the same port serves both the API and the UI.

The server auto-detects `ui/dist` if it exists in the current directory, so running `./mandatum-server` from the repo root after `make build` just works without `--ui`.

### Custom paths and ports

```bash
./mandatum-server --ui /var/www/mandatum \
                  --db /var/data/tasks.db \
                  --rest-port 8080 \
                  --mcp-port 8081
```

Run `./mandatum-server --help` for all options.

---

## Git Workflow

Each task maps to a git branch. The full lifecycle:

```
backlog → [coder claims] → in_progress → [coder requests review] → in_review
       → [reviewer approves] → testing → [tester passes] → docs_needed
       → [docs writer] → done
```

### Step-by-step

**Coder**
```
1. get_next_task          → returns task + suggested branch name
2. setup_worktree         → records worktree path; returns git worktree add commands
   git worktree add .worktrees/<branch> -b <branch>
   cd .worktrees/<branch>
   (OR: git checkout -b <branch> if working in a single checkout)
3. create_branch          → records branch in tracker (if not using setup_worktree)
4. ... write code ...
5. git add . && git commit -m "feat: implement X"
6. record_commit          → records hash + message in tracker
7. (repeat 5–6 as needed)
8. request_review         → moves to in_review, returns reviewer checkout commands
9. set_pr_url             → optional, if you opened a GitHub/GitLab PR
```

**Reviewer**
```
1. get_next_task          → picks from in_review
2. get_review_target      → returns branch name + all commits + git commands
3. setup_worktree         → create an isolated worktree for reviewing
   git worktree add .worktrees/<branch>-review <branch>
   cd .worktrees/<branch>-review
4. git diff main --stat   → inspect changes
5. add_task_comment       → log review notes
6. approve_review         → moves to testing
   OR request_changes     → moves to backlog for coder, clears reviewer assignment, adds feedback
7. git worktree remove .worktrees/<branch>-review  → clean up when done
```

**Tester**
```
1. get_next_task          → picks from testing
2. setup_worktree         → create isolated worktree for testing
   git worktree add .worktrees/<branch>-test <branch>
   cd .worktrees/<branch>-test
3. ... run tests ...
4. record_commit          → record any test commits
5. update_task_status     → "docs_needed" (pass) or "in_progress" (fail with note)
6. git worktree remove .worktrees/<branch>-test  → clean up when done
```

**Docs Writer**
```
1. get_next_task          → picks from docs_needed
2. setup_worktree         → create isolated worktree for docs
   git worktree add .worktrees/<branch>-docs <branch>
   cd .worktrees/<branch>-docs
3. ... write docs ...
4. record_commit          → record docs commit
5. set_output_path        → record docs file path
6. update_task_status     → "done"
7. git worktree remove .worktrees/<branch>-docs  → clean up when done
```

### Branch naming

`get_next_task` suggests a branch name automatically:
```
feature/<first-8-chars-of-task-id>-<slugified-title>
```
e.g. `feature/a1b2c3d4-implement-user-authentication`

### Git worktrees — multiple agents on the same machine

Git worktrees let multiple agents work on different branches **simultaneously** from one repository clone. Each worktree is a separate directory with its own working tree and index, but they all share the same `.git` object store — so no duplication of history.

```bash
# Coder sets up their worktree
git worktree add .worktrees/feature-a1b2c3d4-add-auth -b feature/a1b2c3d4-add-auth

# Reviewer checks out concurrently in a separate worktree
git worktree add .worktrees/feature-a1b2c3d4-add-auth-review feature/a1b2c3d4-add-auth

# List all active worktrees
git worktree list

# Remove when done
git worktree remove .worktrees/feature-a1b2c3d4-add-auth-review
```

The tracker records each agent's `worktree_path` so the UI shows exactly where each agent is working. Add `.worktrees/` to your `.gitignore`.

---

## Connecting AI Agents via MCP

### MCP Configuration

Recommended endpoint for modern MCP clients, including Codex and Claude Code CLI:

```bash
codex mcp add task-tracker --url http://localhost:3002
claude mcp add --transport http task-tracker http://localhost:3002
```

For JSON config-based clients, use:

```json
{
  "mcpServers": {
    "task-tracker": {
      "url": "http://localhost:3002"
    }
  }
}
```

Legacy SSE-only clients can continue using `http://localhost:3002/sse`.

### Using the runner scripts (recommended)

The [agents/claude/](/run/media/plertrood/sponk/src/rust/Mandatum/agents/claude) and [agents/codex/](/run/media/plertrood/sponk/src/rust/Mandatum/agents/codex) directories contain pre-configured runner scripts that handle the MCP connection automatically. See [Running Autonomous Agents](#running-autonomous-agents) above. No manual MCP config is needed when you use those scripts.

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

`assigned_role` is informational only. Claim routing is driven by task status, not by the label.
When a task is claimed, its informational `assigned_role` label is updated to the claiming agent's role.
An agent can only have one active `in_progress` task at a time. If it already has one, `get_next_task` returns that task instead of claiming another.

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

`assigned_role` is stored for UI and reporting purposes. It does not affect which agent can claim the task.

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
{ "agent_id": "coder-alpha" }
```

---

### `create_branch`
Record the git branch you created locally for a task.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "branch_name": "feature/abc12345-add-auth",
  "base_branch": "main"
}
```

### `record_commit`
Record a commit you made locally. Call after every `git commit`.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "hash": "a1b2c3d4e5f6",
  "message": "feat: implement JWT authentication"
}
```

### `request_review`
Mark work done and move to `in_review`. Returns git checkout commands for the reviewer.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "commit_hash": "a1b2c3d4e5f6",
  "note": "All tests passing, ready for review"
}
```

### `get_review_target`
Get the branch, commits, and ready-to-run git commands for reviewing a task.

```json
{ "task_id": "abc-123" }
```

### `approve_review`
Approve the review — moves task to `testing`.

```json
{
  "agent_id": "reviewer-prime",
  "task_id": "abc-123",
  "comment": "LGTM — clean implementation, good test coverage"
}
```

### `request_changes`
Request changes — relabels the task for `coder`, moves it back to `backlog`, clears the reviewer assignment, and records feedback for the coder.

```json
{
  "agent_id": "reviewer-prime",
  "task_id": "abc-123",
  "feedback": "Token expiry not handled — add refresh logic in auth.rs line 42"
}
```

### `set_pr_url`
Record a pull/merge request URL.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "pr_url": "https://github.com/org/repo/pull/42"
}
```

### `setup_worktree`
Record the git worktree path for a task. Returns the `git worktree add` commands to run. Use this instead of (or in addition to) `create_branch` when running multiple agents concurrently — each agent gets an isolated directory to work in.

```json
{
  "agent_id": "coder-alpha",
  "task_id": "abc-123",
  "branch_name": "feature/abc12345-add-auth",
  "worktree_path": ".worktrees/feature-abc12345-add-auth",
  "base_branch": "main"
}
```

Returns:
```json
{
  "message": "Worktree recorded",
  "setup_commands": [
    "git worktree add .worktrees/feature-abc12345-add-auth -b feature/abc12345-add-auth",
    "cd .worktrees/feature-abc12345-add-auth"
  ]
}
```

---

## Agent System Prompts

### Coder Agent
> You are a coding agent connected to a shared task tracker with git integration. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "coder"`
> 2. Call `get_next_task` — returns a task and a suggested branch name
> 3. Call `setup_worktree` with `branch_name` and `worktree_path` (e.g. `.worktrees/<branch>`) — it returns the exact `git worktree add` commands to run. This gives you an isolated working directory so other agents can work concurrently.
> 4. Run the returned `git worktree add` commands, then `cd` into the worktree path
> 5. Implement the task. After each `git commit`, call `record_commit` with the hash and message
> 6. Call `set_output_path` to record the primary file(s) you produced
> 7. Call `request_review` with your HEAD commit hash — moves the task to `in_review`
> 8. Optionally call `set_pr_url` if you opened a pull request
> 9. Repeat from step 2. Send a `heartbeat` every few minutes.

### Reviewer Agent
> You are a code reviewer agent. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "reviewer"`
> 2. Call `get_next_task` — picks a task from `in_review`
> 3. Call `get_review_target` to get the branch name, commit list, and ready-to-run git commands
> 4. Call `setup_worktree` with `worktree_path: ".worktrees/<branch>-review"` to get an isolated checkout for reviewing — run the returned commands
> 5. Inspect changes: `git diff main --stat` then `git diff main`
> 6. Log findings with `add_task_comment`
> 7. If approved: call `approve_review` — moves to `testing`
> 8. If changes needed: call `request_changes` with specific feedback — moves to `backlog` for coder and clears the reviewer assignment
> 9. Run `git worktree remove .worktrees/<branch>-review` to clean up. Repeat. Send a `heartbeat` every few minutes.

### Tester Agent
> You are a QA testing agent. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "tester"`
> 2. Call `get_next_task` — picks from `testing`
> 3. Call `setup_worktree` with `worktree_path: ".worktrees/<branch>-test"` — run the returned commands to get an isolated checkout
> 4. Write and run tests. Call `record_commit` for any test commits you make
> 5. Call `set_output_path` to record your test file path
> 6. If tests pass: call `update_task_status` → `"docs_needed"` with a summary
> 7. If tests fail: call `update_task_status` → `"in_progress"` with failure details
> 8. Run `git worktree remove .worktrees/<branch>-test` to clean up. Repeat. Send a `heartbeat` every few minutes.

### Docs Writer Agent
> You are a technical documentation agent. At the start of each session:
> 1. Call `register_agent` with your `agent_id` and `role: "docs_writer"`
> 2. Call `get_next_task` — picks from `docs_needed`
> 3. Call `setup_worktree` with `worktree_path: ".worktrees/<branch>-docs"` — run the returned commands to get an isolated checkout
> 4. Write documentation for the feature on the same branch
> 5. Call `record_commit` after committing the docs
> 6. Call `set_output_path` to record the docs file path
> 7. Call `update_task_status` → `"done"` with a summary note
> 8. Run `git worktree remove .worktrees/<branch>-docs` to clean up. Repeat. Send a `heartbeat` every few minutes.

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
