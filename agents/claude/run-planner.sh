#!/usr/bin/env bash
# Mandatum planning assistant — interactive Claude session with task-tracker MCP
#
# Usage:
#   ./run-planner.sh                   # plain interactive session
#   ./run-planner.sh plan.md           # interactive, plan pre-loaded in context
#   ./run-planner.sh --auto plan.md    # non-interactive: read plan, create tasks, exit
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MCP_CONFIG="${MCP_CONFIG:-$SCRIPT_DIR/mcp-config.json}"
PROMPT_FILE="$SCRIPT_DIR/planner-prompt.md"

if ! command -v claude &>/dev/null; then
  echo "Error: 'claude' CLI not found. Install it from https://claude.ai/claude-code" >&2
  exit 1
fi

if [[ ! -f "$PROMPT_FILE" ]]; then
  echo "Error: planner-prompt.md not found at $PROMPT_FILE" >&2
  exit 1
fi

# Parse args
AUTO=false
PLAN_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --auto) AUTO=true; shift ;;
    -*) echo "Unknown flag: $1" >&2; exit 1 ;;
    *) PLAN_FILE="$1"; shift ;;
  esac
done

if [[ -n "$PLAN_FILE" && ! -f "$PLAN_FILE" ]]; then
  echo "Error: plan file not found: $PLAN_FILE" >&2
  exit 1
fi

SYSTEM_PROMPT="$(cat "$PROMPT_FILE")"

# Append plan file contents to system prompt if provided
if [[ -n "$PLAN_FILE" ]]; then
  PLAN_CONTENTS="$(cat "$PLAN_FILE")"
  SYSTEM_PROMPT="$SYSTEM_PROMPT

---

## Loaded plan: $(basename "$PLAN_FILE")

The user has provided the following plan. Use it as the basis for task creation when asked.

\`\`\`markdown
$PLAN_CONTENTS
\`\`\`"
fi

if [[ "$AUTO" == true ]]; then
  # Non-interactive: instruct Claude to create tasks from the plan and exit
  if [[ -z "$PLAN_FILE" ]]; then
    echo "Error: --auto requires a plan file" >&2
    exit 1
  fi
  exec claude \
    --mcp-config "$MCP_CONFIG" \
    --append-system-prompt "$SYSTEM_PROMPT" \
    --print "Check the board for existing tasks with list_tasks, then create tasks for every phase and item in the loaded plan. For each task, choose the right assigned_role, write a clear description with acceptance criteria, and set an appropriate priority. After creating all tasks, print a summary of what was added."
else
  exec claude \
    --mcp-config "$MCP_CONFIG" \
    --append-system-prompt "$SYSTEM_PROMPT"
fi
