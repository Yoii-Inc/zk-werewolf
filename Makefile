.PHONY: help install frontend server node stop clean

help:
	@echo "Available targets:"
	@echo "  make install  - Install dependencies for all packages"
	@echo "  make frontend - Start frontend (Next.js) only"
	@echo "  make server   - Start backend server (release mode)"
	@echo "  make node     - Start all zk-mpc-nodes (id=0,1,2) in background"
	@echo "  make stop     - Stop all running services (requires pkill)"
	@echo "  make clean    - Remove build artifacts and node_modules"

# Install dependencies
install:
	@echo "Installing dependencies..."
	cd packages/nextjs && npm install
	cd packages/server && cargo fetch
	cd packages/zk-mpc-node && cargo fetch
	@echo "Dependencies installed!"

# Start only frontend
frontend:
	@echo "Starting frontend with yarn chain and yarn start..."
	yarn chain &
	yarn start

# Start only server (release mode)
server:
	@echo "Starting backend server (release mode)..."
	cd packages/server && ZK_MPC_NODE_0_HTTP=http://localhost:9000 ZK_MPC_NODE_1_HTTP=http://localhost:9001 ZK_MPC_NODE_2_HTTP=http://localhost:9002 cargo run --release

# Start all nodes in background
node:
	@echo "Starting zk-mpc-nodes (id=0,1,2) in background..."
	# Start nodes 1 and 2 in background and discard their output
	cd packages/zk-mpc-node && cargo run --release start --id 1 > /dev/null 2>&1 &
	cd packages/zk-mpc-node && cargo run --release start --id 2 > /dev/null 2>&1 &
	# Start node 0 in foreground so its output is shown
	cd packages/zk-mpc-node && cargo run --release start --id 0
	@echo "Background nodes 1 and 2 started; node 0 has exited."

# Stop all running services
stop:
	@echo "Stopping all services..."
	pkill -f "next dev" || true
	pkill -f "cargo run" || true
	@echo "All services stopped!"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cd packages/nextjs && rm -rf .next node_modules
	cd packages/server && cargo clean
	cd packages/zk-mpc-node && cargo clean
	@echo "Cleanup complete!"
