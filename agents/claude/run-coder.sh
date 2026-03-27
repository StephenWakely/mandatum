#!/usr/bin/env bash
# Runs a coder agent in a continuous loop.
# Usage: ./run-coder.sh [agent-id] [project-dir]
#   or:  PROJECT_DIR=/path/to/repo ./run-coder.sh [agent-id]
#
# project-dir is the git repo the agent works in (defaults to cwd).
# The script itself can live anywhere — paths are resolved absolutely.

set -euo pipefail

AGENT_ID="${AGENT_ID:-${1:-coder-$(hostname)-$$}}"
PROJECT_DIR="${2:-${PROJECT_DIR:-$(pwd)}}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="$SCRIPT_DIR/mcp-config.json"
PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"   # resolve to absolute path
LOG_DIR="${LOG_DIR:-$SCRIPT_DIR/logs}"

# shellcheck source=agents/common-task.sh
source "$SCRIPT_DIR/../common-task.sh"

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/coder-$AGENT_ID.log"

echo "[coder] Starting agent : $AGENT_ID"
echo "[coder] Project dir    : $PROJECT_DIR"
echo "[coder] MCP config     : $MCP_CONFIG"
echo "[coder] Log file       : $LOG_FILE"
echo "[coder] Press Ctrl-C to stop."
echo ""

while true; do
  if stop_requested_rest; then
    echo "[coder/$AGENT_ID] Stop requested. Exiting."
    exit 0
  fi

  echo "[coder/$AGENT_ID] Starting task cycle at $(date '+%H:%M:%S')"

  task_json="$(claim_next_task "coder" 2>>"$LOG_FILE" || true)"
  task_id="$(jq -er '.task.id' <<<"$task_json" 2>/dev/null || true)"
  if [ -z "$task_id" ]; then
    echo "[coder/$AGENT_ID] No task available." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 30
    continue
  fi

  branch_name="$(jq -r '.task.branch_name // .git_instructions.suggested_branch // empty' <<<"$task_json")"
  if [ -z "$branch_name" ]; then
    echo "[coder/$AGENT_ID] Claimed task $task_id but no branch name was available." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  worktree_rel=".worktrees/$(safe_worktree_name "$branch_name")"
  worktree_dir="$(ensure_worktree "$branch_name" "$worktree_rel" "branch" 2>>"$LOG_FILE" || true)"
  if [ -z "$worktree_dir" ]; then
    echo "[coder/$AGENT_ID] Failed to prepare worktree for $branch_name." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  record_worktree_setup "$task_id" "$branch_name" "$worktree_rel"

  title="$(jq -r '.task.title // "(untitled task)"' <<<"$task_json")"
  description="$(jq -r '.task.description // ""' <<<"$task_json")"
  PROMPT="$(cat <<EOF
You are a coder agent. Your agent_id is "$AGENT_ID".
The shell already registered you, claimed the task, and prepared your git worktree.

Project repo root: $PROJECT_DIR
Your worktree: $worktree_dir
Task ID: $task_id
Branch: $branch_name
Title: $title
Description:
$description

Use the configured MCP server via the existing Claude MCP config.
Do not call register_agent, get_next_task, create_branch, or setup_worktree for this task unless you are explicitly repairing broken local state.
Do all file edits and git commands inside "$worktree_dir", not in the repo root.
After each git commit, call record_commit with the hash and message.
Call set_output_path for the primary file or files you produced.
When the task is complete, call request_review with your HEAD commit hash.
Optionally call set_pr_url if you opened a pull request.
Call heartbeat while working.
EOF
)"

  (
    cd "$worktree_dir"
    claude --dangerously-skip-permissions \
      --mcp-config "$MCP_CONFIG" \
      --print "$PROMPT"
  ) 2>&1 | tee -a "$LOG_FILE" || true
  echo ""
  echo "[coder/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
