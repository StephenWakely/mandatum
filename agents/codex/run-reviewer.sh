#!/usr/bin/env bash
# Runs a reviewer agent in a continuous loop.
# Usage: ./run-reviewer.sh [agent-id] [project-dir]
#   or:  PROJECT_DIR=/path/to/repo ./run-reviewer.sh [agent-id]

set -euo pipefail

AGENT_ID="${AGENT_ID:-${1:-reviewer-$(hostname)-$$}}"
PROJECT_DIR="${2:-${PROJECT_DIR:-$(pwd)}}"
PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"

# shellcheck source=agents/codex/common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_codex_runtime

LOG_DIR="${LOG_DIR:-$LOG_DIR_DEFAULT}"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/reviewer-$AGENT_ID.log"

echo "[reviewer] Starting agent : $AGENT_ID"
echo "[reviewer] Project dir    : $PROJECT_DIR"
echo "[reviewer] Codex home     : $HOME/.codex"
echo "[reviewer] MCP server     : $TASK_TRACKER_URL"
echo "[reviewer] Log file       : $LOG_FILE"
echo "[reviewer] Press Ctrl-C to stop."
echo ""

while true; do
  if stop_requested_rest; then
    echo "[reviewer/$AGENT_ID] Stop requested. Exiting."
    exit 0
  fi

  echo "[reviewer/$AGENT_ID] Starting review cycle at $(date '+%H:%M:%S')"

  task_json="$(claim_next_task "reviewer" 2>>"$LOG_FILE" || true)"
  task_id="$(jq -er '.task.id' <<<"$task_json" 2>/dev/null || true)"
  if [ -z "$task_id" ]; then
    echo "[reviewer/$AGENT_ID] No task available." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 30
    continue
  fi

  review_json="$(get_review_target_json "$task_id" 2>>"$LOG_FILE" || true)"
  branch_name="$(jq -r '.review_target.branch // empty' <<<"$review_json")"
  base_branch="$(jq -r '.review_target.base_branch // "main"' <<<"$review_json")"
  if [ -z "$branch_name" ]; then
    echo "[reviewer/$AGENT_ID] Claimed task $task_id but no review branch was recorded." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  worktree_rel=".worktrees/$(safe_worktree_name "$branch_name" "-review")"
  worktree_dir="$(ensure_worktree "$branch_name" "$worktree_rel" "detach" 2>>"$LOG_FILE" || true)"
  if [ -z "$worktree_dir" ]; then
    echo "[reviewer/$AGENT_ID] Failed to prepare review worktree for $branch_name." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  record_worktree_setup "$task_id" "$branch_name" "$worktree_rel" "$base_branch"

  title="$(jq -r '.task.task.title // .task.title // "(untitled task)"' <<<"$review_json")"
  PROMPT="$(cat <<EOF
You are a code reviewer agent. Your agent_id is "$AGENT_ID".
The shell already registered you, claimed the task, fetched the review target, and prepared an isolated review worktree.

Project repo root: $PROJECT_DIR
Your review worktree: $worktree_dir
Task ID: $task_id
Branch under review: $branch_name
Base branch: $base_branch
Title: $title

Use the configured MCP server named "task-tracker".
Do not call register_agent, get_next_task, get_review_target, create_branch, or setup_worktree for this task unless you are explicitly repairing broken local state.
Review the changes in "$worktree_dir".
Log findings with add_task_comment.
If the code is correct and complete, call approve_review.
If changes are needed, call request_changes with specific actionable feedback.
Call heartbeat while working.
EOF
)"

  run_codex_cycle "$worktree_dir" 2>&1 | tee -a "$LOG_FILE" || true
  echo ""
  echo "[reviewer/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
