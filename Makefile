# JS package manager — npm, bun, pnpm, and yarn all work
JAVASCRIPT_RUNTIME    ?= npm
MANDATUM_TARGET_REPO  ?= $(shell pwd)

JS_INSTALL := $(JAVASCRIPT_RUNTIME) install
# Deno users would override this one with deno task build
JS_BUILD   := $(JAVASCRIPT_RUNTIME) run build

export PROJECT_DIR := $(MANDATUM_TARGET_REPO)

.PHONY: build build-server build-ui serve seed clean agents

build: build-server build-ui
	@echo "Build complete ✅"

build-server: | server/Cargo.toml
	cargo build --release --manifest-path $|

build-ui:
	cd ui && $(JS_INSTALL) && $(JS_BUILD)

# Build everything and run as a single process (server serves the UI)
serve: build
	./server/target/release/mandatum-server --ui ui/dist

# Insert sample tasks and agents into the DB
seed:
	@echo "Seeding database..."
	cd server && bash seed.sh

# Run all four agent types in parallel (requires `claude` CLI on PATH)
# MANDATUM_TARGET_REPO defaults to cwd — set it to the repo the agents should work in
agents:
	bash agents/claude/run-all.sh

# Clean build artifacts
clean:
	cd server && cargo clean
	cd ui && $(RM) -r dist node_modules
