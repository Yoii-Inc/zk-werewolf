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

## AWS Deployment

### Prerequisites

1. **AWS CLI** with SSO configured
2. **Terraform** >= 1.2.0
3. **Docker** for building images
4. **SOPS** (optional, for secrets management)
   ```bash
   brew install sops
   ```

### AWS Authentication

```bash
# Login via AWS SSO
aws-sso-util login

# Set AWS profile (required for all AWS operations)
export AWS_PROFILE=yoii-crypto-dev.AWSAdministratorAccess
```

### Terraform Setup

```bash
# Navigate to terraform environment
cd terraform/environments/dev

# Login to Terraform Cloud
terraform login

# Initialize
terraform init

# Plan and apply
terraform plan
terraform apply
```

### Docker Build & Push to ECR

Use the deployment script to build and push Docker images:

```bash
# Deploy all services
./scripts/deploy-to-ecr.sh dev

# Deploy specific service
./scripts/deploy-to-ecr.sh dev frontend
./scripts/deploy-to-ecr.sh dev backend
./scripts/deploy-to-ecr.sh dev mpc-node

# Skip build and push existing images
./scripts/deploy-to-ecr.sh dev --skip-build
```

#### Manual Build (Alternative)

```bash
# Set AWS profile
export AWS_PROFILE=yoii-crypto-dev.AWSAdministratorAccess

# Get ECR repository URLs from Terraform
cd terraform/environments/dev
export FRONTEND_REPO=$(terraform output -raw frontend_repository_url)
export BACKEND_REPO=$(terraform output -raw backend_repository_url)

# Login to ECR
aws ecr get-login-password --region ap-northeast-1 | \
  docker login --username AWS --password-stdin 719037119908.dkr.ecr.ap-northeast-1.amazonaws.com

# Build and push (from project root)
cd ../../..
docker build -f packages/nextjs/Dockerfile -t ${FRONTEND_REPO}:latest .
docker push ${FRONTEND_REPO}:latest
```

### Update ECS Service

After pushing new images, update ECS services:

```bash
# Update all services
./scripts/update-ecs-service.sh dev

# Update specific service
./scripts/update-ecs-service.sh dev frontend
./scripts/update-ecs-service.sh dev backend
```

### Full Deployment Flow

```bash
# 1. Set AWS profile
export AWS_PROFILE=yoii-crypto-dev.AWSAdministratorAccess

# 2. Build and push images
./scripts/deploy-to-ecr.sh dev

# 3. Update ECS services
./scripts/update-ecs-service.sh dev

# 4. Check deployment status
aws ecs describe-services \
  --cluster zk-werewolf-dev \
  --services frontend backend \
  --query 'services[*].{name:serviceName,status:status,running:runningCount,desired:desiredCount}'
```

### Access Deployed Application

```bash
# Get ALB DNS name
cd terraform/environments/dev
terraform output application_url
```

For detailed infrastructure documentation, see [terraform/README.md](terraform/README.md)
