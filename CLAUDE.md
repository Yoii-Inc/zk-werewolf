# ZK Werewolf Project

## Project Overview

ZK Werewolf is a decentralized implementation of the classic Werewolf/Mafia game using zero-knowledge proofs. It ensures game fairness while preserving player role privacy.

## Technology Stack

### Frontend (packages/nextjs)

- Next.js 14.2 + TypeScript
- React 18.3, TailwindCSS, DaisyUI
- Web3 Integration: RainbowKit 2.1.6, wagmi 2.13.4, viem 2.21.54
- State Management: Zustand 5.0
- Authentication: JWT + Custom AuthContext
- Encryption: tweetnacl, @noble/ed25519

### Backend (packages/server)

- Rust (nightly) + Axum 0.7
- WebSocket: tokio-tungstenite
- Database: PostgreSQL 15
- Cache: Redis 7
- Authentication: JWT (jsonwebtoken)
- Password: bcrypt

### Blockchain (packages/foundry)

- Foundry (Forge)
- Solidity Smart Contracts
- Development Environment: Anvil (port 8545)
- OpenZeppelin contracts

### ZK-MPC Nodes (packages/zk-mpc-node)

- Rust
- ZK Framework: zk-mpc (Yoii-Inc)
- Cryptography: ark-bls12-377, ark-marlin
- 3-node configuration (ports 9000-9002)
- Ed25519 key management

## AWS/ECS-based Infrastructure

### Architecture Overview

#### 1. Frontend

- **CloudFront + S3**
  - Next.js static export
  - Edge delivery for low latency
  - OAC (Origin Access Control) for S3 protection

#### 2. Backend API (ECS Fargate)

- **ALB** → **ECS Service**
  - Rust API server
  - Auto-scaling enabled
  - Sticky sessions for WebSocket

#### 3. Database

- **RDS PostgreSQL**
  - Multi-AZ configuration
  - Automated backups (7 days)
  - Performance Insights enabled
- **ElastiCache Redis**
  - Cluster mode disabled
  - Automatic failover

#### 4. MPC Nodes (ECS EC2)

- **ECS Service (EC2 launch type)**
  - 3 individual services
  - Persistent ENI assignment
  - EBS persistent volumes

#### 5. Blockchain

- **Development**: Anvil on EC2
- **Production**: Via Alchemy/Infura

### VPC Network Design

```text
VPC (10.0.0.0/16)
├── Public Subnet (10.0.1.0/24, 10.0.2.0/24) - Multi-AZ
│   ├── ALB
│   └── NAT Gateway
├── Private Subnet (10.0.11.0/24, 10.0.12.0/24) - Multi-AZ
│   ├── ECS Tasks (Fargate)
│   ├── RDS
│   └── ElastiCache
└── MPC Subnet (10.0.21.0/24, 10.0.22.0/24) - Multi-AZ
    └── MPC Nodes (ECS EC2)
```

### Security

#### Security Groups

1. **ALB-SG**: 80/443 from 0.0.0.0/0
2. **Backend-SG**: 8080 from ALB-SG
3. **DB-SG**: 5432 from Backend-SG, MPC-SG
4. **Redis-SG**: 6379 from Backend-SG
5. **MPC-SG**: 9000-9002 inter-node communication

#### Secrets Management

- **AWS Secrets Manager**
  - Database credentials
  - JWT private keys
  - API keys

#### Certificates

- **ACM** for SSL/TLS certificate management

### CI/CD

#### GitHub Actions → ECR → ECS

1. **Build**: Docker image creation
2. **Push**: To ECR
3. **Deploy**: ECS task definition update
4. **Rolling update**

### Monitoring & Logging

#### CloudWatch

- **Container Insights**: ECS metrics
- **Logs**: Application logs
- **Alarms**: Anomaly detection and SNS notifications

#### X-Ray

- Distributed tracing
- Performance analysis

### Cost Optimization

#### Estimated Monthly Cost (Tokyo Region)

- **ECS Fargate**: ~$50-100
- **RDS (t3.small Multi-AZ)**: ~$80
- **ElastiCache (t3.micro)**: ~$25
- **ALB**: ~$25
- **Others**: ~$50
- **Total**: ~$250-300/month

#### Optimization Strategies

1. **Savings Plans** application
2. **Development environment shutdown at night**
3. **Spot Fargate** consideration
4. **S3 Intelligent-Tiering**

### Implementation Steps

#### Phase 1: Infrastructure Foundation

1. VPC/Subnet construction
2. RDS/ElastiCache setup
3. ECR repository creation

#### Phase 2: Application Deployment

1. ECS cluster creation
2. Task definition/service configuration
3. ALB configuration

#### Phase 3: Operations Setup

1. CI/CD pipeline
2. Monitoring & alert configuration
3. Backup verification

## Development Environment

### Docker Compose

All services are containerized and can be started with the following commands:

```bash
make up      # Start all services
make down    # Stop all services
make logs    # Check logs
make restart # Restart services
```

### Service Ports

- Frontend: <http://localhost:3000>
- Backend API: <http://localhost:8080>
- PostgreSQL: localhost:5432
- Redis: localhost:6379
- Anvil (Blockchain): <http://localhost:8545>
- MPC Node 0: <http://localhost:9000>
- MPC Node 1: <http://localhost:9001>
- MPC Node 2: <http://localhost:9002>

## Development Notes

1. **Environment Variables**: Configure `.env` files for each service appropriately
2. **Database Migration**: Automatically executed on first startup
3. **Hot Reload**: Enabled for Rust (cargo-watch) and Next.js
4. **Testing**: Can be run individually for each package

## Troubleshooting

### Common Issues

1. **Port Conflicts**: If ports conflict with existing services, modify docker-compose.yml
2. **Memory Shortage**: Increase Docker Desktop memory allocation (recommended: 8GB+)
3. **Build Errors**: Rust nightly toolchain required

### Debug Commands

```bash
# Check container status
docker ps

# View specific service logs
docker logs zk-werewolf-backend-1

# Execute commands inside container
docker exec -it zk-werewolf-backend-1 bash
```