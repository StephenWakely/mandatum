#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_HOME="${HOME:-}"
RUNTIME_HOME="${CODEX_RUN_HOME:-$DEFAULT_HOME}"
LOG_DIR_DEFAULT="$SCRIPT_DIR/logs"
TASK_TRACKER_URL="${TASK_TRACKER_URL:-http://localhost:3002}"

# shellcheck source=agents/common-task.sh
source "$SCRIPT_DIR/../common-task.sh"

ensure_codex_runtime() {
  if [ -z "$RUNTIME_HOME" ]; then
    echo "ERROR: HOME is not set and CODEX_RUN_HOME was not provided." >&2
    exit 1
  fi

  mkdir -p "$RUNTIME_HOME/.codex"
  export HOME="$RUNTIME_HOME"

  local config_file="$HOME/.codex/config.toml"
  touch "$config_file"

  if ! grep -Fq "[mcp_servers.task-tracker]" "$config_file"; then
    codex mcp add task-tracker --url "$TASK_TRACKER_URL" >/dev/null
  fi

  if ! grep -Fq "[projects.\"$PROJECT_DIR\"]" "$config_file"; then
    {
      printf "\n[projects.\"%s\"]\n" "$PROJECT_DIR"
      printf "trust_level = \"trusted\"\n"
    } >> "$config_file"
  fi
}

run_codex_cycle() {
  local workdir="$1"
  codex exec \
    --dangerously-bypass-approvals-and-sandbox \
    --color never \
    --cd "$workdir" \
    "$PROMPT"
}
