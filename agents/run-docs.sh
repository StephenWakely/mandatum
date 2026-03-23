#!/usr/bin/env bash
# Runs a docs writer agent in a continuous loop.
# Usage: ./run-docs.sh [agent-id]

set -euo pipefail

AGENT_ID="${1:-docs-$(hostname)-$$}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="$SCRIPT_DIR/mcp-config.json"

echo "[docs] Starting agent: $AGENT_ID"
echo "[docs] Press Ctrl-C to stop."
echo ""

PROMPT="You are a technical documentation agent. Your agent_id is \"$AGENT_ID\".

Work through this loop continuously:
1. Call register_agent with agent_id \"$AGENT_ID\" and role \"docs_writer\"
2. Call get_next_task — picks from docs_needed
3. Call setup_worktree with worktree_path \".worktrees/\${branch_name}-docs\" — run the returned commands
4. Read the code and write clear documentation for the feature on the same branch
5. Call record_commit after committing the docs
6. Call set_output_path to record the docs file path
7. Call update_task_status with status \"done\" and a summary note
8. Run git worktree remove on your docs worktree to clean up
9. Send a heartbeat, then go back to step 2

If get_next_task returns no task, call heartbeat and wait 30 seconds before trying again.
Always call heartbeat every 2 minutes while working."

while true; do
  echo "[docs/$AGENT_ID] Starting docs cycle at $(date '+%H:%M:%S')"
  claude --dangerously-skip-permissions \
    --mcp-config "$MCP_CONFIG" \
    --print "$PROMPT" || true
  echo ""
  echo "[docs/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
