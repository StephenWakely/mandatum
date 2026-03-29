#!/usr/bin/env bash
# Runs a docs writer agent in a continuous loop.
# Usage: ./run-docs.sh [agent-id] [project-dir]
#   or:  PROJECT_DIR=/path/to/repo ./run-docs.sh [agent-id]

set -euo pipefail

AGENT_ID="${AGENT_ID:-${1:-docs-$(hostname)-$$}}"
PROJECT_DIR="${2:-${PROJECT_DIR:-$(pwd)}}"
PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"

# shellcheck source=agents/codex/common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_codex_runtime

LOG_DIR="${LOG_DIR:-$LOG_DIR_DEFAULT}"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/docs-$AGENT_ID.log"

echo "[docs] Starting agent : $AGENT_ID"
echo "[docs] Project dir    : $PROJECT_DIR"
echo "[docs] Codex home     : $HOME/.codex"
echo "[docs] MCP server     : $TASK_TRACKER_URL"
echo "[docs] Log file       : $LOG_FILE"
echo "[docs] Press Ctrl-C to stop."
echo ""

register_agent_role "docs_writer"

while true; do
  if stop_requested_rest; then
    echo "[docs/$AGENT_ID] Stop requested. Exiting."
    exit 0
  fi

  echo "[docs/$AGENT_ID] Starting docs cycle at $(date '+%H:%M:%S')"

  task_json="$(claim_next_task "docs_writer" 2>>"$LOG_FILE" || true)"
  task_id="$(jq -r '.task.id // empty' <<<"$task_json" 2>/dev/null || true)"
  if [ -z "$task_id" ]; then
    echo "[docs/$AGENT_ID] No task available." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 30
    continue
  fi

  branch_name="$(jq -r '.task.branch_name // empty' <<<"$task_json")"
  if [ -z "$branch_name" ]; then
    echo "[docs/$AGENT_ID] Claimed task $task_id but no branch was recorded." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  worktree_rel=".worktrees/$(safe_worktree_name "$branch_name" "-docs")"
  worktree_dir="$(ensure_worktree "$branch_name" "$worktree_rel" "branch" 2>>"$LOG_FILE" || true)"
  if [ -z "$worktree_dir" ]; then
    echo "[docs/$AGENT_ID] Failed to prepare docs worktree for $branch_name." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  record_worktree_setup "$task_id" "$branch_name" "$worktree_rel"

  title="$(jq -r '.task.title // "(untitled task)"' <<<"$task_json")"
  description="$(jq -r '.task.description // ""' <<<"$task_json")"
  PROMPT="$(cat <<EOF
You are a technical documentation agent. Your agent_id is "$AGENT_ID".
The shell already registered you, claimed the task, and prepared your worktree.

Project repo root: $PROJECT_DIR
Your docs worktree: $worktree_dir
Task ID: $task_id
Branch: $branch_name
Title: $title
Description:
$description

Use the configured MCP server named "task-tracker".
Do not call register_agent, get_next_task, create_branch, or setup_worktree for this task unless you are explicitly repairing broken local state.
Write or update documentation in "$worktree_dir".
Call record_commit after each commit you make.
Call set_output_path for the docs file or files you produced.
When documentation is complete, call update_task_status with status "done" and a concise summary.
Call heartbeat while working.
EOF
)"

  run_codex_cycle "$worktree_dir" 2>&1 | tee -a "$LOG_FILE" || true
  echo ""
  echo "[docs/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
