# AGENTS.md

This file defines how autonomous coding agents should operate in this repository.

## Project Summary

Mandatum is a multi-agent task tracker:

- `server/` contains the Rust backend, REST API, MCP/SSE server, SQLite access, and git/task orchestration.
- `ui/` contains the React + TypeScript frontend.
- `agents/claude/` contains Claude Code runner scripts.
- `agents/codex/` contains Codex runner scripts.
- `Makefile` provides the main developer entry points.

Read [README.md](/run/media/plertrood/sponk/src/rust/Mandatum/README.md) before making broad changes. Treat it as the primary product and workflow reference.

## Operating Rules

- Make the smallest correct change that fully solves the task.
- Preserve existing behavior unless the task explicitly requires changing it.
- Prefer editing existing code over introducing new modules or abstractions.
- Keep patches easy to review. Avoid mixing refactors with behavior changes unless necessary.
- Do not rewrite unrelated formatting, naming, or structure.
- If a task spans backend and frontend, keep the API contract explicit and update both sides coherently.
- When changing workflow or behavior that affects users or other agents, update `README.md` and this file if needed.

## Repo Workflow

Use the documented local commands:

- Backend dev: `cd server && cargo run`
- Frontend dev: `cd ui && npm run dev`
- Full dev loop: `make dev` when available in the environment described by the README
- Production build: `make build`
- Single-process serve: `make serve`
- Seed sample data: `make seed`

Before finishing substantial changes:

- Run `cargo fmt` in `server/`
- Run `cargo check` in `server/`
- Run `cargo test` in `server/` if tests exist or if you add them
- Run `npm run build` in `ui/` for frontend-impacting changes

If you cannot run a validation step, say so explicitly in your final handoff.

## Agent Role Guidance

When operating through Mandatum’s own task system:

- Register with the correct role first.
- Claim work only through `get_next_task`.
- Use worktrees when the workflow expects isolated branches.
- Record commits with `record_commit`.
- Move tasks through the documented lifecycle:
  `backlog -> in_progress -> in_review -> testing -> docs_needed -> done`
- Keep task comments concise and factual.
- Send heartbeats while working so tasks do not get reaped as stale.

When editing agent runner scripts:

- Keep Claude and Codex role workflows equivalent unless there is a tool-specific limitation.
- Preserve parity between corresponding scripts in `agents/claude/` and `agents/codex/`.
- If one runner family gains a new step, document and port it to the other family unless divergence is intentional.
- Codex runners use the caller's normal `HOME` by default and auto-register the local `task-tracker` MCP server there.
- If isolation is needed, use `CODEX_RUN_HOME` to point the runner at a separate Codex home.

## Rust Rules: Strict Tiger Style

All Rust changes in `server/` must follow strict Tiger Style. Treat these rules as mandatory.

### Core principles

- Choose simple, explicit code over clever code.
- Make control flow obvious from top to bottom.
- Keep data flow visible. Avoid hidden mutation and surprising side effects.
- Prefer boring, robust code over compact or abstract code.
- Write code that a reviewer can verify quickly by inspection.

### Structure

- Prefer free functions and straightforward structs over deep trait hierarchies.
- Introduce abstractions only when they remove clear duplication or isolate a real boundary.
- Keep functions small enough to understand in one pass.
- Group related logic together; do not scatter one workflow across many tiny files.
- Avoid premature generalization.

### Error handling

- Never use `unwrap()` or `expect()` in non-test code unless process termination is the intended and justified behavior at the top level.
- Propagate errors with context instead of hiding them.
- Prefer explicit `Result` handling over silent fallback behavior.
- Fail early on invalid state.

### State and concurrency

- Minimize mutable state.
- Make ownership and lifetimes easy to reason about.
- Keep async boundaries clear and necessary.
- Do not spawn background tasks unless the lifecycle and failure behavior are obvious.

### Data and APIs

- Use precise types and names.
- Avoid boolean parameters when an enum or dedicated type would be clearer.
- Keep serialization and database field changes deliberate; maintain compatibility unless the task requires a schema change.
- When changing persistent data or protocol behavior, update all affected call sites and documentation in the same patch.

### Style

- Follow `rustfmt`, but do not stop there. Code must also be readable without relying on formatting.
- Prefer `match` and `if let` only when they clarify the branching; do not compress logic for terseness.
- Avoid deeply nested conditionals; flatten with early returns where appropriate.
- Avoid macros when ordinary Rust is clearer.
- Comments should explain intent or invariants, not restate the code.

### Performance and safety

- Avoid unnecessary allocation, cloning, and string churn in hot paths.
- Do not add `unsafe` without an explicit, documented justification. Prefer not to use it at all.
- Optimize only when there is a concrete reason, but do not introduce obviously wasteful work.

## Frontend Rules

For `ui/` changes:

- Keep the current React + TypeScript structure unless the task requires otherwise.
- Preserve the existing visual language unless asked to redesign it.
- Keep types explicit across API boundaries.
- Prefer small, readable components over indirection-heavy patterns.

## Change Hygiene

- If you add a new command, env var, port, workflow step, or dependency, document it in [README.md](/run/media/plertrood/sponk/src/rust/Mandatum/README.md).
- If you change task-state semantics, MCP tools, or agent expectations, update this file too.
- Do not leave dead code, commented-out code, or speculative scaffolding behind.

## Handoff Expectations

Final handoffs should include:

- What changed
- What was validated
- Any constraints, follow-ups, or risks

Keep handoffs concise, concrete, and technically precise.
