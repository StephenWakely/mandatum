#!/bin/bash
# Seed script — inserts sample tasks and agents into the DB
set -e
BASE="http://localhost:3001"
MCP="http://localhost:3002"

echo "Seeding tasks..."

post_task() {
  curl -sf -X POST "$BASE/api/tasks" \
    -H "Content-Type: application/json" \
    -d "$1" > /dev/null && echo "  ✓ $2"
}

post_task '{"title":"Implement user authentication","description":"Add JWT-based auth with login/logout endpoints and refresh token support","priority":"high","assigned_role":"coder","tags":["auth","backend","security"]}' "auth task"
post_task '{"title":"Write API documentation","description":"Document all REST endpoints with request/response examples using OpenAPI spec","priority":"medium","assigned_role":"docs_writer","tags":["docs","api"]}' "docs task"
post_task '{"title":"Fix memory leak in worker pool","description":"Workers are not being cleaned up properly on shutdown — profiler shows steady growth","priority":"critical","assigned_role":"coder","tags":["bug","performance","memory"]}' "bug task"
post_task '{"title":"Review PR: rate limiting middleware","description":"Code review for the token bucket rate limiting middleware PR #142","priority":"high","assigned_role":"reviewer","tags":["review","security","middleware"]}' "review task"
post_task '{"title":"Integration tests for checkout flow","description":"Write end-to-end tests covering the full payment checkout process including edge cases","priority":"high","assigned_role":"tester","tags":["testing","payments","e2e"]}' "test task"
post_task '{"title":"Refactor database connection pooling","description":"Replace custom pool with bb8 crate for better reliability and metrics","priority":"medium","assigned_role":"coder","tags":["refactor","database"]}' "refactor task"
post_task '{"title":"Update onboarding guide","description":"Rewrite the getting started guide for new developers joining the project","priority":"low","assigned_role":"docs_writer","tags":["docs","onboarding"]}' "onboarding docs"
post_task '{"title":"Performance benchmark suite","description":"Create benchmarks for hot paths in the HTTP request handler using criterion","priority":"medium","assigned_role":"tester","tags":["performance","benchmarks"]}' "benchmark task"

# Move some tasks to non-backlog statuses directly via PATCH
echo ""
echo "Updating task statuses..."

# Get task IDs
TASKS=$(curl -sf "$BASE/api/tasks" | python3 -c "import sys,json; tasks=json.load(sys.stdin); [print(t['id'],t['title']) for t in tasks]" 2>/dev/null || true)

if command -v python3 &>/dev/null; then
  IDS=$(curl -sf "$BASE/api/tasks" | python3 -c "
import sys, json
tasks = json.load(sys.stdin)
for i, t in enumerate(tasks):
    print(i, t['id'])
" 2>/dev/null)

  # Update a few tasks to different statuses
  T0=$(curl -sf "$BASE/api/tasks" | python3 -c "import sys,json; t=json.load(sys.stdin); print(t[0]['id'] if len(t)>0 else '')" 2>/dev/null)
  T1=$(curl -sf "$BASE/api/tasks" | python3 -c "import sys,json; t=json.load(sys.stdin); print(t[1]['id'] if len(t)>1 else '')" 2>/dev/null)
  T2=$(curl -sf "$BASE/api/tasks" | python3 -c "import sys,json; t=json.load(sys.stdin); print(t[2]['id'] if len(t)>2 else '')" 2>/dev/null)
  T3=$(curl -sf "$BASE/api/tasks" | python3 -c "import sys,json; t=json.load(sys.stdin); print(t[3]['id'] if len(t)>3 else '')" 2>/dev/null)
  T4=$(curl -sf "$BASE/api/tasks" | python3 -c "import sys,json; t=json.load(sys.stdin); print(t[4]['id'] if len(t)>4 else '')" 2>/dev/null)

  [ -n "$T0" ] && curl -sf -X PATCH "$BASE/api/tasks/$T0" -H "Content-Type: application/json" -d '{"status":"in_progress","assigned_agent_id":"coder-alpha"}' > /dev/null && echo "  ✓ moved task 0 → in_progress"
  [ -n "$T1" ] && curl -sf -X PATCH "$BASE/api/tasks/$T1" -H "Content-Type: application/json" -d '{"status":"in_review"}' > /dev/null && echo "  ✓ moved task 1 → in_review"
  [ -n "$T2" ] && curl -sf -X PATCH "$BASE/api/tasks/$T2" -H "Content-Type: application/json" -d '{"status":"in_progress","assigned_agent_id":"coder-beta"}' > /dev/null && echo "  ✓ moved task 2 → in_progress"
  [ -n "$T3" ] && curl -sf -X PATCH "$BASE/api/tasks/$T3" -H "Content-Type: application/json" -d '{"status":"done"}' > /dev/null && echo "  ✓ moved task 3 → done"
  [ -n "$T4" ] && curl -sf -X PATCH "$BASE/api/tasks/$T4" -H "Content-Type: application/json" -d '{"status":"testing"}' > /dev/null && echo "  ✓ moved task 4 → testing"
fi

echo ""
echo "Registering agents..."

mcp_call() {
  curl -sf -X POST "$MCP/message" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"register_agent\",\"arguments\":{\"agent_id\":\"$1\",\"role\":\"$2\"}}}" > /dev/null \
    && echo "  ✓ registered $1 ($2)" \
    || echo "  ✗ failed to register $1 (is server running on $MCP?)"
}

mcp_call "coder-alpha"    "coder"
mcp_call "coder-beta"     "coder"
mcp_call "reviewer-prime" "reviewer"
mcp_call "tester-main"    "tester"
mcp_call "docs-writer-1"  "docs_writer"

echo ""
echo "Done! Open http://localhost:5173 to see the board."
