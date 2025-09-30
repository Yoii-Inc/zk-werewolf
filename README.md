# ZK Werewolf

A decentralized werewolf game using zero-knowledge proofs and secure multi-party computation (MPC) to ensure player privacy and game integrity.

## Development Setup

### Prerequisites

- Docker & Docker Compose

### Getting Started

1. Start all services:
   ```bash
   docker compose up --build
   ```

2. Access the services:
   - Frontend: http://localhost:3000  
   - Backend: http://localhost:8080
   - Blockchain: http://localhost:8545
   - MPC Nodes: http://localhost:9000, http://localhost:9001, http://localhost:9002

### Development Commands

```bash
# Start development environment
docker compose up -d

# View logs
docker compose logs -f

# Stop services  
docker compose down

# Clean rebuild
docker compose down -v
docker compose up --build
```

### Project Structure

- `packages/foundry/` - Smart contracts (Solidity + Foundry)
- `packages/nextjs/` - Frontend (Next.js + React)
- `packages/server/` - Backend API (Rust + Axum)
- `packages/zk-mpc-node/` - MPC node (Rust)
- `packages/mpc-circuits/` - ZK circuits
- `packages/mpc-algebra-wasm/` - WebAssembly bindings
