#!/usr/bin/env bash
# Runs a coder agent in a continuous loop.
# Usage: ./run-coder.sh [agent-id]
#   agent-id defaults to coder-<hostname>-<pid>

set -euo pipefail

AGENT_ID="${1:-coder-$(hostname)-$$}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="$SCRIPT_DIR/mcp-config.json"

echo "[coder] Starting agent: $AGENT_ID"
echo "[coder] MCP config: $MCP_CONFIG"
echo "[coder] Press Ctrl-C to stop."
echo ""

PROMPT="You are a coder agent. Your agent_id is \"$AGENT_ID\".

Work through this loop continuously:
1. Call register_agent with agent_id \"$AGENT_ID\" and role \"coder\"
2. Call get_next_task — it returns a task and a suggested branch name
3. Call setup_worktree with the suggested branch_name and worktree_path \".worktrees/\${branch_name}\" — run the returned git worktree add commands
4. Implement the task fully. After each git commit, call record_commit with the hash and message
5. Call set_output_path to record the primary file(s) you produced
6. Call request_review with your HEAD commit hash
7. Optionally call set_pr_url if you opened a pull request
8. Send a heartbeat, then go back to step 2 and claim the next task

If get_next_task returns no task, call heartbeat and wait 30 seconds before trying again.
Always call heartbeat every 2 minutes while working.
If a task has no branch yet and no worktree is appropriate, use create_branch instead."

while true; do
  echo "[coder/$AGENT_ID] Starting task cycle at $(date '+%H:%M:%S')"
  claude --dangerously-skip-permissions \
    --mcp-config "$MCP_CONFIG" \
    --print "$PROMPT" || true
  echo ""
  echo "[coder/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
