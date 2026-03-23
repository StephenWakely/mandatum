#!/usr/bin/env bash
# Launch one of each agent type in parallel.
# Each runs in its own terminal tab/process.
# Usage: ./run-all.sh
# Ctrl-C kills all agents.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cleanup() {
  echo ""
  echo "Stopping all agents..."
  kill 0
}
trap cleanup INT TERM

echo "Starting all agents. Press Ctrl-C to stop all."
echo ""

"$SCRIPT_DIR/run-coder.sh"    "coder-1"    2>&1 | sed 's/^/[coder]    /' &
"$SCRIPT_DIR/run-reviewer.sh" "reviewer-1" 2>&1 | sed 's/^/[reviewer] /' &
"$SCRIPT_DIR/run-tester.sh"   "tester-1"   2>&1 | sed 's/^/[tester]   /' &
"$SCRIPT_DIR/run-docs.sh"     "docs-1"     2>&1 | sed 's/^/[docs]     /' &

wait
