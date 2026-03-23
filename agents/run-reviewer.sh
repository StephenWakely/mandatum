#!/usr/bin/env bash
# Runs a reviewer agent in a continuous loop.
# Usage: ./run-reviewer.sh [agent-id] [project-dir]
#   or:  PROJECT_DIR=/path/to/repo ./run-reviewer.sh [agent-id]

set -euo pipefail

AGENT_ID="${AGENT_ID:-${1:-reviewer-$(hostname)-$$}}"
PROJECT_DIR="${2:-${PROJECT_DIR:-$(pwd)}}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="$SCRIPT_DIR/mcp-config.json"
PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"

echo "[reviewer] Starting agent : $AGENT_ID"
echo "[reviewer] Project dir    : $PROJECT_DIR"
echo "[reviewer] MCP config     : $MCP_CONFIG"
echo "[reviewer] Press Ctrl-C to stop."
echo ""

PROMPT="You are a code reviewer agent. Your agent_id is \"$AGENT_ID\".
You are working in the git repository at: $PROJECT_DIR

Work through this loop continuously:
1. Call register_agent with agent_id \"$AGENT_ID\" and role \"reviewer\"
2. Call get_next_task — picks a task from in_review
3. Call get_review_target to get the branch name, commit list, and git commands
4. Call setup_worktree with worktree_path \".worktrees/\${branch_name}-review\" inside $PROJECT_DIR — run the returned commands
5. Inspect the changes: git log and git diff against the base branch
6. Log your findings with add_task_comment
7. If the code is correct and complete: call approve_review
   If changes are needed: call request_changes with specific, actionable feedback
8. Run git worktree remove on your review worktree to clean up
9. Send a heartbeat, then go back to step 2

If get_next_task returns no task, call heartbeat and wait 30 seconds before trying again.
Always call heartbeat every 2 minutes while working."

cd "$PROJECT_DIR"

while true; do
  echo "[reviewer/$AGENT_ID] Starting review cycle at $(date '+%H:%M:%S')"
  claude --dangerously-skip-permissions \
    --mcp-config "$MCP_CONFIG" \
    --print "$PROMPT" || true
  echo ""
  echo "[reviewer/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
