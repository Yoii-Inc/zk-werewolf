# ZK Werewolf

A decentralized werewolf game using zero-knowledge proofs and secure multi-party computation (MPC) to ensure player privacy and game integrity.

### Key Features

- **Privacy-Preserving**: Uses zero-knowledge proofs to hide player roles and actions
- **Decentralized**: Runs on blockchain for transparent game state management
- **Secure**: Implements MPC for secret sharing and secure vote computation
- **Fair Play**: Cryptographic guarantees prevent cheating and ensure game rules
- **Web3 Integration**: Seamless wallet connection and on-chain interactions

## Development Setup

### Prerequisites

- Docker (24.0.0 or later)
- Docker Compose (v2.0.0 or later)

That's it! All other development tools (Rust, Node.js, Foundry) are handled within Docker containers.

### Getting Started

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/zk-werewolf.git
   cd zk-werewolf
   ```

2. Initialize and start all services:

   ```bash
   make init
   ```

   This will:

   - Install project dependencies
   - Build Docker images
   - Start all services in development mode

3. Access the services:
   - Frontend: http://localhost:3000
   - Backend API: http://localhost:8080
   - Blockchain node: http://localhost:8545
   - MPC nodes: http://localhost:9000-9002

### Development Commands

- `make dev` - Start all services in development mode
- `make build` - Build all Docker images
- `make down` - Stop all services
- `make clean` - Clean up containers and build artifacts
- `make clean-images` - Remove all project Docker images
- `make clean-all` - Deep clean including Docker cache
- `make test` - Run all tests
- `make help` - Show all available commands

### Development Workflow

The development environment is set up with hot reloading:

- Frontend (Next.js) changes will automatically refresh the browser
- Backend (Rust) changes will trigger automatic rebuilds
- Smart contract changes require manual redeployment: `make deploy-contracts`

### Project Structure

- `packages/`
  - `foundry/` - Smart contracts and blockchain node
  - `nextjs/` - Frontend application
  - `server/` - Backend API server
  - `zk-mpc-node/` - MPC node implementation
  - `mpc-circuits/` - Zero-knowledge circuit implementations
  - `mpc-algebra-wasm/` - WebAssembly bindings for MPC operations
