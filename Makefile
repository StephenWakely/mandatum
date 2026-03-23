.PHONY: dev build serve seed clean agents

# Start both server and UI concurrently
dev:
	@echo "Starting Mandatum dev servers..."
	@trap 'kill %1 %2 2>/dev/null; exit' INT; \
	(cd server && cargo run 2>&1 | sed 's/^/[server] /') & \
	(cd ui && npm run dev 2>&1 | sed 's/^/[ui] /') & \
	wait

# Build Rust server (release) and React UI
build:
	@echo "Building Rust server..."
	cd server && cargo build --release
	@echo "Building React UI..."
	cd ui && npm run build
	@echo "Build complete."

# Build everything and run as a single process (server serves the UI)
serve: build
	@echo "Starting Mandatum (single process, UI on :3001)..."
	./server/target/release/mandatum-server --ui ui/dist

# Insert sample tasks and agents into the DB
seed:
	@echo "Seeding database..."
	@chmod +x server/seed.sh
	@cd server && bash seed.sh

# Run all four agent types in parallel (requires `claude` CLI on PATH)
# PROJECT_DIR defaults to cwd — set it to the repo the agents should work in:
#   make agents PROJECT_DIR=/path/to/your/project
#   make -C /path/to/mandatum agents PROJECT_DIR=/path/to/your/project
PROJECT_DIR ?= $(shell pwd)
agents:
	@echo "Starting all agents in $(PROJECT_DIR)..."
	@chmod +x agents/*.sh
	@PROJECT_DIR="$(PROJECT_DIR)" agents/run-all.sh

# Clean build artifacts
clean:
	cd server && cargo clean
	cd ui && rm -rf dist node_modules
