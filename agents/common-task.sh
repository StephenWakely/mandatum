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
