#!/usr/bin/env bash
# Runs a tester agent in a continuous loop.
# Usage: ./run-tester.sh [agent-id] [project-dir]
#   or:  PROJECT_DIR=/path/to/repo ./run-tester.sh [agent-id]

set -euo pipefail

AGENT_ID="${AGENT_ID:-${1:-tester-$(hostname)-$$}}"
PROJECT_DIR="${2:-${PROJECT_DIR:-$(pwd)}}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="$SCRIPT_DIR/mcp-config.json"
PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"
LOG_DIR="${LOG_DIR:-$SCRIPT_DIR/logs}"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/tester-$AGENT_ID.log"

echo "[tester] Starting agent : $AGENT_ID"
echo "[tester] Project dir    : $PROJECT_DIR"
echo "[tester] MCP config     : $MCP_CONFIG"
echo "[tester] Log file       : $LOG_FILE"
echo "[tester] Press Ctrl-C to stop."
echo ""

PROMPT="You are a QA testing agent. Your agent_id is \"$AGENT_ID\".
You are working in the git repository at: $PROJECT_DIR

Work through this loop continuously:
1. Call register_agent with agent_id \"$AGENT_ID\" and role \"tester\"
2. Call get_next_task — picks from testing
3. Call setup_worktree with worktree_path \".worktrees/\${branch_name}-test\" inside $PROJECT_DIR — run the returned commands
4. Write and run tests against the code. Call record_commit for any test commits
5. Call set_output_path to record your test file path
6. If tests pass: call update_task_status with status \"docs_needed\" and a summary
   If tests fail: call update_task_status with status \"in_progress\" and the failure details
7. Run git worktree remove on your test worktree to clean up
8. Send a heartbeat, then go back to step 2

If get_next_task returns no task, call heartbeat and wait 30 seconds before trying again.
Always call heartbeat every 2 minutes while working."

cd "$PROJECT_DIR"

while true; do
  echo "[tester/$AGENT_ID] Starting test cycle at $(date '+%H:%M:%S')"
  claude --dangerously-skip-permissions \
    --mcp-config "$MCP_CONFIG" \
    --print "$PROMPT" 2>&1 | tee -a "$LOG_FILE" || true
  echo ""
  echo "[tester/$AGENT_ID] Cycle complete. Restarting in 10s..."
  sleep 10
done
