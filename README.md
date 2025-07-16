# 🏗 Scaffold-ETH 2

<h4 align="center">
  <a href="https://docs.scaffoldeth.io">Documentation</a> |
  <a href="https://scaffoldeth.io">Website</a>
</h4>

🧪 An open-source, up-to-date toolkit for building decentralized applications (dapps) on the Ethereum blockchain. It's designed to make it easier for developers to create and deploy smart contracts and build user interfaces that interact with those contracts.

⚙️ Built using NextJS, RainbowKit, Foundry, Wagmi, Viem, and Typescript.

## System Architecture

```mermaid
graph TD
  classDef important fill:#ff4444,stroke:#ff0000,stroke-width:2px;
  subgraph User_Communication
    UserA["User A<br>(Browser/App)"] <-- WebSocket/Waku/Libp2p --> UserB["User B<br>(Browser/App)"]
    UserA --> FrontendA["Frontend A<br>(React, Vue, Unity, etc.)"]:::important
    UserB --> FrontendB["Frontend B<br>(React, Vue, Unity, etc.)"]:::important
  end

  FrontendA -- On-chain Txn --> BlockchainNodeA[Blockchain<br>Node/RPC]
  FrontendB -- On-chain Txn --> BlockchainNodeB[Blockchain<br>Node/RPC]

  subgraph On_Chain
    BlockchainNodeA -- "Smart Contract Call" --> SmartContract["Message Logic<br>(Smart Contract)"]:::important
    BlockchainNodeB -- "Smart Contract Call" --> SmartContract
    SmartContract -- "Store Hash/Event Log" --> BlockchainStorage["Blockchain Storage<br>(Ethereum/Layer2)"]
  end

  subgraph Off_Chain
    IPFS["Message Storage<br>(IPFS/Filecoin)"]
    API["Off-chain API<br>(Rust/WebSocket)"]:::important
    MPC1["MPC Server 1<br>(Rust)"]
    MPC2["MPC Server 2<br>(Rust)"]
    MPC3["MPC Server 3<br>(Rust)"]
    DataIndexing["Data Indexing<br> (The Graph)"]
  end

  SmartContract -- "Store Reference" --> IPFS
  UserA -- "Real-time Messaging" --> API
  UserB -- "Real-time Messaging" --> API
  API --> MPC1
  API --> MPC2
  API --> MPC3
  IPFS --> DataIndexing
```

## Deployment Architecture

The system consists of the following components:

1. **Frontend (Next.js)**

   - Static site hosting (e.g., Vercel, Netlify)
   - Serves the React application
   - Handles client-side routing and UI rendering

2. **Backend (Rust)**

   - Containerized deployment
   - Handles WebSocket connections
   - Manages real-time communication
   - Uses PostgreSQL for data storage

3. **zk-mpc-node**

   - Distributed node deployment
   - Handles zero-knowledge proof computations
   - State management through Redis

4. **Smart Contracts**
   - Deployed on Ethereum/Layer2 networks
   - Interacts with backend via RPC
   - Uses IPFS for metadata storage

## Database Architecture

The system uses a combination of PostgreSQL and Redis:

1. **PostgreSQL**

   - User management and authentication
   - Persistent data storage
   - Row Level Security (RLS) enabled

2. **Redis**
   - Game state management
   - Real-time communication
   - Temporary data storage

### Database Schema

```sql
-- Create users table in PostgreSQL
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY,
  username TEXT NOT NULL,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
  last_login TIMESTAMP WITH TIME ZONE
);

-- Set up RLS policies
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- Allow anonymous users to create new users
CREATE POLICY "Allow user registration" ON users
  FOR INSERT TO anon
  WITH CHECK (true);

-- Allow users to access only their own data
CREATE POLICY "Users can view their own data" ON users
  FOR SELECT TO authenticated
  USING (auth.uid() = id);

CREATE POLICY "Users can update their own data" ON users
  FOR UPDATE TO authenticated
  USING (auth.uid() = id);

-- Allow email-based user search
CREATE POLICY "Allow email-based user search" ON users
  FOR SELECT TO anon
  USING (true);

-- Create index
CREATE INDEX IF NOT EXISTS users_email_idx ON users (email);
```

## Environment Configuration

### Required Environment Variables

1. **Database**

   ```bash
   DATABASE_URL=postgresql://user:password@localhost:5432/dbname
   REDIS_URL=redis://localhost:6379
   ```

2. **Smart Contracts**

   ```bash
   PRIVATE_KEY=your-deployer-private-key   # Contract deployment private key
   RPC_URL=your-ethereum-rpc-url           # Testnet/Mainnet RPC URL
   ```

3. **Authentication**

   ```bash
   JWT_SECRET=your-secure-jwt-secret       # JWT signing secret
   ```

4. **MPC Nodes**

   ```bash
   MPC_SECRET_KEY=your-mpc-secret-key      # MPC node secret key
   MPC_PUBLIC_KEY=your-mpc-public-key      # MPC node public key
   ```

5. **Debug Settings**
   ```bash
   DEBUG_MODE=false
   DEBUG_VERBOSE_LOGGING=false
   DEBUG_SHOW_PLAYER_ROLES=false
   DEBUG_AUTO_ADVANCE_PHASES=false
   DEBUG_PHASE_DURATION_SECONDS=30
   DEBUG_RANDOM_ROLE=false
   DEBUG_CREATE_CRYPTO_PARAMETERS=false
   ```

### Environment-Specific Configuration

- **Development**

  - Local PostgreSQL and Redis
  - All DEBUG\_\* variables set to true
  - Testnet RPC URL

- **Staging**

  - Managed PostgreSQL and Redis
  - Limited DEBUG\_\* variables enabled
  - Testnet RPC URL

- **Production**
  - Managed PostgreSQL and Redis with high availability
  - All DEBUG\_\* variables set to false
  - Mainnet RPC URL

## Deployment Process

1. **Frontend Deployment**

   ```bash
   cd packages/nextjs
   yarn build
   yarn deploy
   ```

2. **Backend Deployment**

   ```bash
   cd packages/server
   cargo build --release
   docker build -t zk-werewolf-backend .
   docker push zk-werewolf-backend
   ```

3. **zk-mpc-node Deployment**

   ```bash
   cd packages/zk-mpc-node
   cargo build --release
   docker build -t zk-werewolf-mpc .
   docker push zk-werewolf-mpc
   ```

4. **Smart Contract Deployment**
   ```bash
   cd packages/foundry
   yarn deploy
   ```

## CI/CD Pipeline

The project uses GitHub Actions for continuous deployment:

1. Triggers on push to main branch
2. Sets up Node.js and Rust environments
3. Runs tests
4. Builds Docker images
5. Deploys smart contracts
6. Deploys backend services
7. Deploys frontend
8. Runs integration tests
9. Updates environment variables

## Security Considerations

1. **Secret Management**

   - All secrets are encrypted
   - Regular rotation of production keys
   - Minimal required permissions

2. **Access Control**

   - Row Level Security in PostgreSQL
   - JWT-based authentication
   - Secure WebSocket connections

3. **Data Protection**
   - Encrypted data transmission
   - Secure password hashing
   - Regular security audits

## Monitoring and Maintenance

1. **Health Checks**

   - Regular endpoint monitoring
   - Service health metrics
   - Performance tracking

2. **Logging**

   - Centralized logging system
   - Application logs
   - Audit logs

3. **Backup Strategy**
   - Regular database backups
   - State recovery procedures
   - Disaster recovery plan

- ✅ **Contract Hot Reload**: Your frontend auto-adapts to your smart contract as you edit it.
- 🪝 **[Custom hooks](https://docs.scaffoldeth.io/hooks/)**: Collection of React hooks wrapper around [wagmi](https://wagmi.sh/) to simplify interactions with smart contracts with typescript autocompletion.
- 🧱 [**Components**](https://docs.scaffoldeth.io/components/): Collection of common web3 components to quickly build your frontend.
- 🔥 **Burner Wallet & Local Faucet**: Quickly test your application with a burner wallet and local faucet.
- 🔐 **Integration with Wallet Providers**: Connect to different wallet providers and interact with the Ethereum network.

![Debug Contracts tab](https://github.com/scaffold-eth/scaffold-eth-2/assets/55535804/b237af0c-5027-4849-a5c1-2e31495cccb1)

## Requirements

Before you begin, you need to install the following tools:

- [Node (>= v18.18)](https://nodejs.org/en/download/)
- Yarn ([v1](https://classic.yarnpkg.com/en/docs/install/) or [v2+](https://yarnpkg.com/getting-started/install))
- [Git](https://git-scm.com/downloads)

## Quickstart

To get started with Scaffold-ETH 2, follow the steps below:

1. Install dependencies if it was skipped in CLI:

```
cd my-dapp-example
yarn install
```

2. Run a local network in the first terminal:

```
yarn chain
```

This command starts a local Ethereum network using Foundry. The network runs on your local machine and can be used for testing and development. You can customize the network configuration in `packages/foundry/foundry.toml`.

3. On a second terminal, deploy the test contract:

```
yarn deploy
```

This command deploys a test smart contract to the local network. The contract is located in `packages/foundry/contracts` and can be modified to suit your needs. The `yarn deploy` command uses the deploy script located in `packages/foundry/script` to deploy the contract to the network. You can also customize the deploy script.

4. On a third terminal, start your NextJS app:

```
yarn start
```

Visit your app on: `http://localhost:3000`. You can interact with your smart contract using the `Debug Contracts` page. You can tweak the app config in `packages/nextjs/scaffold.config.ts`.

Run smart contract test with `yarn foundry:test`

- Edit your smart contracts in `packages/foundry/contracts`
- Edit your frontend homepage at `packages/nextjs/app/page.tsx`. For guidance on [routing](https://nextjs.org/docs/app/building-your-application/routing/defining-routes) and configuring [pages/layouts](https://nextjs.org/docs/app/building-your-application/routing/pages-and-layouts) checkout the Next.js documentation.
- Edit your deployment scripts in `packages/foundry/script`

## Repository Structure

This project follows a monorepo structure using Yarn workspaces. Here's an overview of the main directories and their purposes:

```text
.
├── packages/                # Main packages directory
│   ├── zk-mpc-node/         # Zero-knowledge MPC node implementation
│   ├── nextjs/              # Frontend application (Next.js)
│   ├── server/              # Backend server (Rust)
│   └── foundry/             # Smart contract development environment
├── .husky/                  # Git hooks configuration
├── .yarn/                   # Yarn configuration
└── .github/                 # GitHub Actions configuration
```

### Package Details

- **zk-mpc-node/**

  - Zero-knowledge proof and MPC implementation
  - Handles secure computation and verification

- **nextjs/**

  - Frontend user interface
  - Built with Next.js and React
  - Integrates with smart contracts and backend

- **server/**

  - Backend API server
  - Implemented in Rust
  - Handles business logic and data management

- **foundry/**
  - Smart contract development environment
  - Contract compilation and deployment
  - Testing and verification tools

### Development Tools

- **.husky/**

  - Git hooks for pre-commit checks
  - Code quality enforcement

- **.yarn/**

  - Yarn package manager configuration
  - Workspace management

- **.github/**
  - CI/CD pipeline configuration
  - Automated testing and deployment

## Documentation

Visit our [docs](https://docs.scaffoldeth.io) to learn how to start building with Scaffold-ETH 2.

To know more about its features, check out our [website](https://scaffoldeth.io).

## Contributing to Scaffold-ETH 2

We welcome contributions to Scaffold-ETH 2!

Please see [CONTRIBUTING.MD](https://github.com/scaffold-eth/scaffold-eth-2/blob/main/CONTRIBUTING.md) for more information and guidelines for contributing to Scaffold-ETH 2.
