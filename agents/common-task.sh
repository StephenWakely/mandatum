#!/usr/bin/env bash

set -euo pipefail

MANDATUM_MCP_URL="${MANDATUM_MCP_URL:-http://localhost:3002}"
MANDATUM_REST_URL="${MANDATUM_REST_URL:-http://localhost:3001}"

mcp_tool_call() {
  local tool_name="$1"
  local args_json="${2-}"
  [ -z "$args_json" ] && args_json='{}'
  local payload response

  payload="$(printf '%s' "$args_json" | jq -c \
    --arg name "$tool_name" \
    '{jsonrpc:"2.0", id:1, method:"tools/call", params:{name:$name, arguments:.}}')"

  response="$(curl -sf \
    -H 'Content-Type: application/json' \
    -d "$payload" \
    "$MANDATUM_MCP_URL/")" || {
    echo "ERROR: Cannot reach Mandatum server at $MANDATUM_MCP_URL — is it running? (make serve)" >&2
    return 1
  }

  if jq -e '.error != null' >/dev/null <<<"$response"; then
    jq -r '.error.message' <<<"$response" >&2
    return 1
  fi

  jq -cer '.result.content[0].text | fromjson' <<<"$response"
}

register_agent_role() {
  local role="$1"
  mcp_tool_call "register_agent" "$(jq -cn --arg agent_id "$AGENT_ID" --arg role "$role" '{agent_id:$agent_id, role:$role}')" >/dev/null
}

heartbeat_agent() {
  mcp_tool_call "heartbeat" "$(jq -cn --arg agent_id "$AGENT_ID" '{agent_id:$agent_id}')" >/dev/null || true
}

claim_next_task() {
  local role="$1"
  register_agent_role "$role"
  mcp_tool_call "get_next_task" "$(jq -cn --arg agent_id "$AGENT_ID" --arg role "$role" '{agent_id:$agent_id, role:$role}')"
}

get_review_target_json() {
  local task_id="$1"
  mcp_tool_call "get_review_target" "$(jq -cn --arg task_id "$task_id" '{task_id:$task_id}')"
}

record_worktree_setup() {
  local task_id="$1"
  local branch_name="$2"
  local worktree_path="$3"
  local base_branch="${4:-}"
  local args

  if [ -n "$base_branch" ]; then
    args="$(jq -cn \
      --arg agent_id "$AGENT_ID" \
      --arg task_id "$task_id" \
      --arg branch_name "$branch_name" \
      --arg worktree_path "$worktree_path" \
      --arg base_branch "$base_branch" \
      '{agent_id:$agent_id, task_id:$task_id, branch_name:$branch_name, worktree_path:$worktree_path, base_branch:$base_branch}')"
  else
    args="$(jq -cn \
      --arg agent_id "$AGENT_ID" \
      --arg task_id "$task_id" \
      --arg branch_name "$branch_name" \
      --arg worktree_path "$worktree_path" \
      '{agent_id:$agent_id, task_id:$task_id, branch_name:$branch_name, worktree_path:$worktree_path}')"
  fi

  mcp_tool_call "setup_worktree" "$args" >/dev/null
}

stop_requested_rest() {
  curl -sf "$MANDATUM_REST_URL/api/agents" 2>/dev/null | \
    jq -e --arg id "$AGENT_ID" '.[] | select(.agent_id == $id) | .stop_requested == true' \
    > /dev/null 2>&1
}

CLAUDE_STREAM_FILTER='
  if .type == "assistant" then
    (.message.content // [])[]? |
      if .type == "tool_use" then "→ \(.name)\((.input // {} | tostring) as $i | if ($i | length) > 0 then "(\($i | .[:120]))" else "" end)"
      elif .type == "text" then "» \((.text // "") | gsub("\n"; " ") | .[:200])"
      else empty end
  elif .type == "user" then
    (.message.content // [])[]? |
      if .type == "tool_result" then "  ↳ \((.content // "" | if type == "array" then tostring else . end) | tostring | gsub("\n"; " ") | .[:120])"
      else empty end
  elif .type == "result" then "■ done (\(.duration_ms // 0)ms)"
  else empty end
'

# Filter stream-json output from `claude --output-format stream-json` into
# one human-readable line per event. Lines that aren't JSON (errors, debug
# output, etc.) are passed through verbatim.
claude_stream_filter() {
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    if [ "${line:0:1}" = "{" ]; then
      printf '%s\n' "$line" | jq -rc "$CLAUDE_STREAM_FILTER" 2>/dev/null
    else
      printf '%s\n' "$line"
    fi
  done
}

# Print an end-of-run summary from a captured stream-json file: duration,
# turns, cost, token usage, and a histogram of tool calls (MCP + builtin).
agent_run_summary() {
  local f="$1"
  [ -f "$f" ] || return 0
  echo
  echo "─── run summary ───"
  jq -r '
    select(.type == "result") |
    "duration   : \(((.duration_ms // 0) / 1000) | tostring | .[:6])s  turns=\(.num_turns // "?")  cost=$\(.total_cost_usd // 0)",
    "tokens     : in=\(.usage.input_tokens // 0)  out=\(.usage.output_tokens // 0)  cache_create=\(.usage.cache_creation_input_tokens // 0)  cache_read=\(.usage.cache_read_input_tokens // 0)"
  ' "$f" 2>/dev/null
  local tools
  tools="$(jq -r 'select(.type == "assistant") | .message.content[]? | select(.type == "tool_use") | .name' "$f" 2>/dev/null | sort | uniq -c | sort -rn)"
  if [ -n "$tools" ]; then
    echo "tool calls :"
    echo "$tools" | sed 's/^/  /'
  fi
  echo "───────────────────"
}

dump_agent_diagnostics() {
  local label="${1:-agent}"
  local worktree="${2:-}"
  echo "─── diagnostics ($label) ───"
  echo "host       : $(hostname 2>/dev/null || echo unknown)  user=$(whoami) pwd=$(pwd)"
  echo "claude     : $(command -v claude || echo MISSING)  version=$(claude --version 2>/dev/null || echo n/a)"
  echo "MCP URL    : ${MANDATUM_MCP_URL:-http://localhost:3002}"
  echo "REST URL   : ${MANDATUM_REST_URL:-http://localhost:3001}"
  echo "MCP config : $MCP_CONFIG (exists=$([ -f "$MCP_CONFIG" ] && echo yes || echo no))"
  echo "auth env   : API_KEY=$([ -n "${ANTHROPIC_API_KEY:-}" ] && echo set || echo unset)" \
       " AUTH_TOKEN=$([ -n "${ANTHROPIC_AUTH_TOKEN:-}" ] && echo set || echo unset)" \
       " BASE_URL=${ANTHROPIC_BASE_URL:-unset}"
  echo "REST reach : $(curl -sf -o /dev/null -w '%{http_code}' "${MANDATUM_REST_URL:-http://localhost:3001}/api/info" 2>&1 || echo unreachable)"
  if [ -n "$worktree" ]; then
    echo "worktree   : $worktree (exists=$([ -d "$worktree" ] && echo yes || echo no))"
    if [ -d "$worktree" ]; then
      echo "             branch=$(git -C "$worktree" rev-parse --abbrev-ref HEAD 2>/dev/null || echo n/a) head=$(git -C "$worktree" rev-parse --short HEAD 2>/dev/null || echo n/a)"
    fi
  fi
  echo "────────────────────────────"
}

safe_worktree_name() {
  local branch_name="$1"
  local suffix="${2:-}"
  printf '%s%s' "${branch_name//\//__}" "$suffix"
}

find_existing_branch_worktree() {
  local branch_name="$1"

  git -C "$PROJECT_DIR" worktree list --porcelain | awk -v branch="refs/heads/$branch_name" '
    /^worktree / { path = substr($0, 10) }
    /^branch / && $2 == branch { print path }
  '
}

ensure_worktree() {
  local branch_name="$1"
  local worktree_rel="$2"
  local mode="${3:-branch}"
  local abs_worktree="$PROJECT_DIR/$worktree_rel"
  local existing_branch_path=""

  mkdir -p "$(dirname "$abs_worktree")"

  if git -C "$abs_worktree" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    # For detached-HEAD worktrees (reviewer/tester), advance to the current branch tip
    # so the agent always sees the latest commits, not whatever was there last cycle.
    if [ "$mode" = "detach" ]; then
      local tip
      tip="$(git -C "$PROJECT_DIR" rev-parse "$branch_name" 2>/dev/null || true)"
      if [ -n "$tip" ]; then
        git -C "$abs_worktree" checkout --detach "$tip" >/dev/null 2>&1 || true
      fi
    fi
    printf '%s\n' "$abs_worktree"
    return 0
  fi

  if [ -e "$abs_worktree" ] && [ ! -d "$abs_worktree" ]; then
    echo "Worktree path exists and is not a directory: $abs_worktree" >&2
    return 1
  fi

  if [ "$mode" = "branch" ]; then
    existing_branch_path="$(find_existing_branch_worktree "$branch_name" | head -n 1 || true)"
    if [ -n "$existing_branch_path" ] && [ "$existing_branch_path" != "$PROJECT_DIR" ]; then
      printf '%s\n' "$existing_branch_path"
      return 0
    fi

    if git -C "$PROJECT_DIR" show-ref --verify --quiet "refs/heads/$branch_name"; then
      git -C "$PROJECT_DIR" worktree add "$abs_worktree" "$branch_name" 1>&2
    else
      git -C "$PROJECT_DIR" worktree add "$abs_worktree" -b "$branch_name" 1>&2
    fi
  else
    git -C "$PROJECT_DIR" worktree add --detach "$abs_worktree" "$branch_name" 1>&2
  fi

  printf '%s\n' "$abs_worktree"
}
