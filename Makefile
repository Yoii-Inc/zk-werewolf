.PHONY: help install frontend server node stop clean groth16-setup groth16-export-verifier

ROLE_ASSIGNMENT_GROTH16_PK_PATH ?= packages/zk-mpc-node/data/groth16/role_assignment_max5_v1.pk
ANONYMOUS_VOTING_GROTH16_PK_PATH ?= packages/zk-mpc-node/data/groth16/anonymous_voting_max5_v1.pk
DIVINATION_GROTH16_PK_PATH ?= packages/zk-mpc-node/data/groth16/divination_max5_v1.pk
WINNING_JUDGEMENT_GROTH16_PK_PATH ?= packages/zk-mpc-node/data/groth16/winning_judgement_max5_v1.pk
KEY_PUBLICIZE_GROTH16_PK_PATH ?= packages/zk-mpc-node/data/groth16/key_publicize_max5_v1.pk

help:
	@echo "Available targets:"
	@echo "  make install  - Install dependencies for all packages"
	@echo "  make frontend - Start frontend (Next.js) only"
	@echo "  make server   - Start backend server (release mode)"
	@echo "  make node     - Start all zk-mpc-nodes (id=0,1,2) in background"
	@echo "  make stop     - Stop all running services (requires pkill)"
	@echo "  make clean    - Remove build artifacts and node_modules"
	@echo "  make groth16-setup - Generate Groth16 setup artifacts for all circuits"
	@echo "  make groth16-export-verifier - Export verifier contracts for all circuits from proving keys"

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
	@for p in 8000 8001 8002 9000 9001 9002; do \
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

groth16-setup:
	cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin role_assignment_groth16_setup
	cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin anonymous_voting_groth16_setup
	cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin divination_groth16_setup
	cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin winning_judgement_groth16_setup
	cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin key_publicize_groth16_setup

groth16-export-verifier:
	ROLE_ASSIGNMENT_GROTH16_PK_PATH=$(ROLE_ASSIGNMENT_GROTH16_PK_PATH) \
		cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin role_assignment_groth16_verifier_export
	ANONYMOUS_VOTING_GROTH16_PK_PATH=$(ANONYMOUS_VOTING_GROTH16_PK_PATH) \
		cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin anonymous_voting_groth16_verifier_export
	DIVINATION_GROTH16_PK_PATH=$(DIVINATION_GROTH16_PK_PATH) \
		cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin divination_groth16_verifier_export
	WINNING_JUDGEMENT_GROTH16_PK_PATH=$(WINNING_JUDGEMENT_GROTH16_PK_PATH) \
		cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin winning_judgement_groth16_verifier_export
	KEY_PUBLICIZE_GROTH16_PK_PATH=$(KEY_PUBLICIZE_GROTH16_PK_PATH) \
		cargo run --manifest-path packages/arkworks-solidity-verifier/Cargo.toml --release --bin key_publicize_groth16_verifier_export
