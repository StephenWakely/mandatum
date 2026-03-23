#!/usr/bin/env bash
# Runs a tester agent in a continuous loop.
# Usage: ./run-tester.sh [agent-id]

set -euo pipefail

AGENT_ID="${1:-tester-$(hostname)-$$}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="$SCRIPT_DIR/mcp-config.json"

echo "[tester] Starting agent: $AGENT_ID"
echo "[tester] Press Ctrl-C to stop."
echo ""

PROMPT="You are a QA testing agent. Your agent_id is \"$AGENT_ID\".

Work through this loop continuously:
1. Call register_agent with agent_id \"$AGENT_ID\" and role \"tester\"
2. Call get_next_task — picks from testing
3. Call setup_worktree with worktree_path \".worktrees/\${branch_name}-test\" — run the returned commands
4. Write and run tests against the code. Call record_commit for any test commits
5. Call set_output_path to record your test file path
6. If tests pass: call update_task_status with status \"docs_needed\" and a summary
   If tests fail: call update_task_status with status \"in_progress\" and the failure details
7. Run git worktree remove on your test worktree to clean up
8. Send a heartbeat, then go back to step 2

If get_next_task returns no task, call heartbeat and wait 30 seconds before trying again.
Always call heartbeat every 2 minutes while working."

while true; do
  echo "[tester/$AGENT_ID] Starting test cycle at $(date '+%H:%M:%S')"
  claude --dangerously-skip-permissions \
    --mcp-config "$MCP_CONFIG" \
    --print "$PROMPT" || true
  echo ""
  echo "[tester/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
