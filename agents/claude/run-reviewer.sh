#!/usr/bin/env bash
# Runs a reviewer agent in a continuous loop.
# Usage: ./run-reviewer.sh [agent-id] [project-dir]
#   or:  PROJECT_DIR=/path/to/repo ./run-reviewer.sh [agent-id]

set -euo pipefail

AGENT_ID="${AGENT_ID:-${1:-reviewer-$(hostname)-$$}}"
PROJECT_DIR="${2:-${PROJECT_DIR:-$(pwd)}}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="${MCP_CONFIG:-$SCRIPT_DIR/mcp-config.json}"
PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"
LOG_DIR="${LOG_DIR:-$SCRIPT_DIR/logs}"

# shellcheck source=agents/common-task.sh
source "$SCRIPT_DIR/../common-task.sh"

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/reviewer-$AGENT_ID.log"

echo "[reviewer] Starting agent : $AGENT_ID"
echo "[reviewer] Project dir    : $PROJECT_DIR"
echo "[reviewer] MCP config     : $MCP_CONFIG"
echo "[reviewer] Log file       : $LOG_FILE"
echo "[reviewer] Press Ctrl-C to stop."
echo ""

register_agent_role "reviewer"

while true; do
  if stop_requested_rest; then
    echo "[reviewer/$AGENT_ID] Stop requested. Exiting."
    exit 0
  fi

  echo "[reviewer/$AGENT_ID] Starting review cycle at $(date '+%H:%M:%S')"

  task_json="$(claim_next_task "reviewer" 2>>"$LOG_FILE" || true)"
  task_id="$(jq -r '.task.id // empty' <<<"$task_json" 2>/dev/null || true)"
  if [ -z "$task_id" ]; then
    echo "[reviewer/$AGENT_ID] No task available." | tee -a "$LOG_FILE"
    if [ "${MANDATUM_ONCE:-0}" = "1" ]; then exit 0; fi
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
  worktree_dir="$(ensure_worktree "$branch_name" "$worktree_rel" "branch" 2>>"$LOG_FILE" || true)"
  if [ -z "$worktree_dir" ]; then
    echo "[reviewer/$AGENT_ID] Failed to prepare review worktree for $branch_name." | tee -a "$LOG_FILE"
    heartbeat_agent
    sleep 10
    continue
  fi

  record_worktree_setup "$task_id" "$branch_name" "$worktree_rel" "$base_branch"

  title="$(jq -r '.task.task.title // .task.title // "(untitled task)"' <<<"$review_json")"

  # Extract prior feedback that the coder was asked to fix
  prior_feedback="$(jq -r '
    .review_target.prior_feedback
    | if length > 0 then
        to_entries
        | map("[\(.key + 1)] \(.value)")
        | join("\n\n")
      else "" end
  ' <<<"$review_json" 2>/dev/null || true)"

  PRIOR_FEEDBACK_SECTION=""
  if [ -n "$prior_feedback" ]; then
    PRIOR_FEEDBACK_SECTION="$(cat <<FEEDBACK

MANDATORY CHECKLIST — This task was previously returned for changes. Before doing anything else, verify each of the following points is fixed in the code. Use grep, cat, or git show to confirm with your own eyes. Do NOT approve if any point is unresolved.

$prior_feedback

For each point above:
- Find the relevant file and line — confirm the fix is present in the actual code
- Find the test the coder claims proves the fix — run it and confirm it passes
- If the fix is missing, call request_changes immediately citing the unresolved point(s)
- If the fix is present but has no test, write the test yourself now, commit it, then continue
- Only proceed to a general review once all prior feedback is confirmed fixed and tested
FEEDBACK
)"
  fi

  EXTRA_BLOCK=""
  if [ -n "${ADDITIONAL_INSTRUCTIONS:-}" ]; then
    EXTRA_BLOCK="
Additional instructions:
${ADDITIONAL_INSTRUCTIONS}
"
  fi

  PROMPT="$(cat <<EOF
You are a code reviewer agent. Your agent_id is "$AGENT_ID".
The shell already registered you, claimed the task, fetched the review target, and prepared an isolated review worktree.

Project repo root: $PROJECT_DIR
Your review worktree: $worktree_dir
Task ID: $task_id
Branch under review: $branch_name
Base branch: $base_branch
Title: $title
$PRIOR_FEEDBACK_SECTION$EXTRA_BLOCK
Use the configured MCP server via the existing Claude MCP config.
Do not call register_agent, get_next_task, get_review_target, create_branch, or setup_worktree for this task unless you are explicitly repairing broken local state.
Review the changes in "$worktree_dir".
Log findings with add_task_comment.
If you write any tests, commit them with git and call record_commit with the hash and message.
If the code is correct and complete, call approve_review.
If changes are needed, call request_changes with specific actionable feedback.
Call heartbeat while working.
EOF
)"

  dump_agent_diagnostics "reviewer/$AGENT_ID" "$worktree_dir"
  STREAM_TMP="$(mktemp)"
  (
    cd "$worktree_dir"
    claude --dangerously-skip-permissions \
      --mcp-config "$MCP_CONFIG" \
      --output-format stream-json --verbose \
      --print "$PROMPT" 2>&1
  ) | tee "$STREAM_TMP" | claude_stream_filter | tee -a "$LOG_FILE" || true
  agent_run_summary "$STREAM_TMP" | tee -a "$LOG_FILE"
  rm -f "$STREAM_TMP"
  echo ""
  if [ "${MANDATUM_ONCE:-0}" = "1" ]; then
    echo "[reviewer/$AGENT_ID] One-shot mode: exiting."
    exit 0
  fi
  echo "[reviewer/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
