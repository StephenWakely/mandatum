# JS package manager — npm, bun, pnpm, and yarn all work
JSRUNTIME  := npm
PROJECT_DIR ?= $(shell pwd)

.PHONY: build build-server build-ui serve seed clean agents

build: build-server build-ui
	@echo "Build complete."

build-server:
	@echo "Building Rust server..."
	cd server && cargo build --release

build-ui:
	@echo "Building React UI..."
	cd ui && $(JSRUNTIME) install && $(JSRUNTIME) run build

# Build everything and run as a single process (server serves the UI)
serve: build
	@echo "Starting Mandatum (single process, UI on :3001)..."
	./server/target/release/mandatum-server --ui ui/dist

# Insert sample tasks and agents into the DB
seed:
	@echo "Seeding database..."
	cd server && bash seed.sh

# Run all four agent types in parallel (requires `claude` CLI on PATH)
# PROJECT_DIR defaults to cwd — set it to the repo the agents should work in:
#   make agents PROJECT_DIR=/path/to/your/project
#   make -C /path/to/mandatum agents PROJECT_DIR=/path/to/your/project
agents:
	@echo "Starting all agents in $(PROJECT_DIR)..."
	@PROJECT_DIR="$(PROJECT_DIR)" agents/run-all.sh

# Clean build artifacts
clean:
	cd server && cargo clean
	cd ui && rm -rf dist node_modules
