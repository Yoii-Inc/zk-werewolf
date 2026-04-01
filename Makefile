.PHONY: help install frontend server node node-small stop stop-node clean groth16-setup docker-up docker-up-detached docker-down

NODE_PORTS := 8000 8001 8002 9000 9001 9002
ALL_SERVICE_PORTS := 3000 8080 $(NODE_PORTS)

help:
	@echo "Available targets:"
	@echo "  make install  - Install dependencies for all packages"
	@echo "  make frontend - Start frontend (Next.js) only"
	@echo "  make server   - Start backend server (release mode)"
	@echo "  make node     - Start all zk-mpc-nodes (id=0,1,2) in background"
	@echo "  make node-small - Start zk-mpc-nodes with Groth16 keys up to 5 players"
	@echo "  make stop-node - Stop only zk-mpc-node processes by listening ports"
	@echo "  make stop     - Stop all running services by listening ports"
	@echo "  make clean    - Remove build artifacts and node_modules"
	@echo "  make groth16-setup - Generate Groth16 setup artifacts for all circuits"
	@echo "  make docker-up - Build and start local stack with docker compose (foreground)"
	@echo "  make docker-up-detached - Build and start local stack with docker compose (background)"
	@echo "  make docker-down - Stop docker compose stack"

# Install dependencies
install:
	@echo "Installing dependencies..."
	yarn install
	cargo fetch
	@echo "Dependencies installed!"

# Start only frontend
frontend:
	@echo "Building mpc-algebra-wasm, starting chain, and starting frontend..."
	cd packages/mpc-algebra-wasm && wasm-pack build --target web --out-dir pkg-web
# 	yarn chain &
	yarn start

# Start only server (release mode)
server:
	@echo "Starting backend server (release mode)..."
	cd packages/server && ZK_MPC_NODE_0_HTTP=http://localhost:9000 ZK_MPC_NODE_1_HTTP=http://localhost:9001 ZK_MPC_NODE_2_HTTP=http://localhost:9002 cargo run --release

# Start all nodes in background
node:
	@echo "Starting zk-mpc-nodes (id=0,1,2) in background..."
	@for p in $(NODE_PORTS); do \
		if lsof -nP -iTCP:$$p -sTCP:LISTEN >/dev/null 2>&1; then \
			echo "Port $$p is already in use. Run 'make stop' or kill the process using that port."; \
			exit 1; \
		fi; \
	done
	# Start nodes 1 and 2 in background and discard their output
	cd packages/zk-mpc-node && cargo run --release --bin zk-mpc-node start --id 1 &
	cd packages/zk-mpc-node && cargo run --release --bin zk-mpc-node start --id 2 &
	# Start node 0 in foreground so its output is shown
	cd packages/zk-mpc-node && cargo run --release --bin zk-mpc-node start --id 0
	@echo "Background nodes 1 and 2 started; node 0 has exited."

# Start all nodes with up-to-5 Groth16 proving keys in background
node-small:
	@echo "Starting zk-mpc-nodes (id=0,1,2) with GROTH16_DATA_DIR=data/groth16-up-to-5 ..."
	@for p in $(NODE_PORTS); do \
		if lsof -nP -iTCP:$$p -sTCP:LISTEN >/dev/null 2>&1; then \
			echo "Port $$p is already in use. Run 'make stop' or kill the process using that port."; \
			exit 1; \
		fi; \
	done
	# Start nodes 1 and 2 in background and discard their output
	cd packages/zk-mpc-node && env GROTH16_DATA_DIR=data/groth16-up-to-5 cargo run --release --bin zk-mpc-node start --id 1 &
	cd packages/zk-mpc-node && env GROTH16_DATA_DIR=data/groth16-up-to-5 cargo run --release --bin zk-mpc-node start --id 2 &
	# Start node 0 in foreground so its output is shown
	cd packages/zk-mpc-node && env GROTH16_DATA_DIR=data/groth16-up-to-5 cargo run --release --bin zk-mpc-node start --id 0
	@echo "Background nodes 1 and 2 started; node 0 has exited."

# Stop only zk-mpc-node services
stop-node:
	@echo "Stopping zk-mpc-node services by listening ports..."
	@for p in $(NODE_PORTS); do \
		pids=$$(lsof -tiTCP:$$p -sTCP:LISTEN 2>/dev/null || true); \
		if [ -n "$$pids" ]; then \
			echo "Stopping node process(es) on port $$p: $$pids"; \
			kill $$pids 2>/dev/null || true; \
		fi; \
	done
	@sleep 1
	@for p in $(NODE_PORTS); do \
		pids=$$(lsof -tiTCP:$$p -sTCP:LISTEN 2>/dev/null || true); \
		if [ -n "$$pids" ]; then \
			echo "Force stopping node process(es) on port $$p: $$pids"; \
			kill -9 $$pids 2>/dev/null || true; \
		fi; \
	done
	@echo "Node port-based stop completed."

# Stop all running services
stop:
	@echo "Stopping services by listening ports..."
	@for p in $(ALL_SERVICE_PORTS); do \
		pids=$$(lsof -tiTCP:$$p -sTCP:LISTEN 2>/dev/null || true); \
		if [ -n "$$pids" ]; then \
			echo "Stopping process(es) on port $$p: $$pids"; \
			kill $$pids 2>/dev/null || true; \
		fi; \
	done
	@sleep 1
	@for p in $(ALL_SERVICE_PORTS); do \
		pids=$$(lsof -tiTCP:$$p -sTCP:LISTEN 2>/dev/null || true); \
		if [ -n "$$pids" ]; then \
			echo "Force stopping process(es) on port $$p: $$pids"; \
			kill -9 $$pids 2>/dev/null || true; \
		fi; \
	done
	@echo "Port-based stop completed."

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cd packages/nextjs && rm -rf .next node_modules
	cd packages/server && cargo clean
	cd packages/zk-mpc-node && cargo clean
	@echo "Cleanup complete!"

groth16-setup:
	cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin multi_profile_groth16_setup

docker-up:
	docker compose up --build

docker-up-detached:
	docker compose up --build -d

docker-down:
	docker compose down --remove-orphans
