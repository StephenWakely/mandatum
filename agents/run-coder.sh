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
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/coder-$AGENT_ID.log"

echo "[coder] Starting agent : $AGENT_ID"
echo "[coder] Project dir    : $PROJECT_DIR"
echo "[coder] MCP config     : $MCP_CONFIG"
echo "[coder] Log file       : $LOG_FILE"
echo "[coder] Press Ctrl-C to stop."
echo ""

PROMPT="You are a coder agent. Your agent_id is \"$AGENT_ID\".
You are working in the git repository at: $PROJECT_DIR

Work through this loop continuously:
1. Call register_agent with agent_id \"$AGENT_ID\" and role \"coder\"
2. Call get_next_task — it returns a task and a suggested branch name
3. Call setup_worktree with the suggested branch_name and worktree_path \".worktrees/\${branch_name}\" — run the returned git worktree add commands inside $PROJECT_DIR
4. Implement the task fully. After each git commit, call record_commit with the hash and message
5. Call set_output_path to record the primary file(s) you produced
6. Call request_review with your HEAD commit hash
7. Optionally call set_pr_url if you opened a pull request
8. Send a heartbeat, then go back to step 2 and claim the next task

If get_next_task returns no task, call heartbeat and wait 30 seconds before trying again.
Always call heartbeat every 2 minutes while working.
If a task has no branch yet and no worktree is appropriate, use create_branch instead."

cd "$PROJECT_DIR"

while true; do
  # Check if stop has been requested for this agent
  if curl -sf "http://localhost:3001/api/agents" 2>/dev/null | \
      jq -e --arg id "$AGENT_ID" '.[] | select(.agent_id == $id) | .stop_requested == true' > /dev/null 2>&1; then
    echo "[coder/$AGENT_ID] Stop requested. Exiting."
    exit 0
  fi
  echo "[coder/$AGENT_ID] Starting task cycle at $(date '+%H:%M:%S')"
  claude --dangerously-skip-permissions \
    --mcp-config "$MCP_CONFIG" \
    --print "$PROMPT" 2>&1 | tee -a "$LOG_FILE" || true
  echo ""
  echo "[coder/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
