# ZK Werewolf Terraform Infrastructure

This Terraform configuration deploys the ZK Werewolf application to AWS ECS using Fargate and various AWS services.

## Architecture

- **Frontend**: CloudFront + S3 for static hosting (planned)
- **Backend API**: ECS Fargate behind Application Load Balancer
- **MPC Nodes**: 3 ECS Fargate services (planned)
- **Database**: Supabase Cloud (external)
- **Service Discovery**: AWS Cloud Map for internal service communication
- **Networking**: VPC with public/private subnets across 2 AZs
  - **Dev environment**: Public subnets only (no NAT Gateway for cost savings)
  - **Prod environment**: Private subnets with NAT Gateway (recommended)
- **Secrets Management**: SOPS with AWS KMS for encrypted secrets in Git

## Prerequisites

1. **AWS CLI** configured with appropriate credentials

   ```bash
   aws configure --profile yoii-crypto-dev
   # or use AWS SSO
   aws-sso-util login
   ```

2. **Terraform** >= 1.2.0

3. **Terraform Cloud** account (for remote state)
   - Organization: "Yoii"
   - Workspace execution mode: "Local"

4. **Docker** for building images

5. **Supabase project** created
   - Note the Supabase URL and API key
   - Database connection string (if needed)

6. **SOPS** for encrypted secrets management (optional)

   ```bash
   brew install sops  # macOS
   ```

## Setup

### 1. Configure Terraform Variables

Navigate to the environment directory:

```bash
cd terraform/environments/dev
```

The `terraform.tfvars` file contains basic AWS configuration:

```hcl
AWS_DEFAULT_REGION = "ap-northeast-1"
AWS_ACCOUNT_ID     = "719037119908"  # Update for your account
```

### 2. Set Environment Variables for Secrets

Set sensitive information as environment variables (not in Git):

```bash
export TF_VAR_supabase_url="https://your-project.supabase.co"
export TF_VAR_supabase_key="your-supabase-anon-key"
export TF_VAR_jwt_secret="your-jwt-secret-key"
```

Or use SOPS for encrypted secrets (recommended):

```bash
# Encrypt secrets file
sops --kms "alias/zk-werewolf-dev-sops-key" --encrypt secrets.yaml > secrets.enc.yaml

# Decrypt and export
eval $(sops --decrypt secrets.enc.yaml | yq eval -o=shell)
```

### 3. Authenticate with Terraform Cloud

```bash
terraform login
```

### 4. Set AWS Profile

```bash
export AWS_PROFILE=yoii-crypto-dev.AWSAdministratorAccess
```

### 5. Initialize Terraform

```bash
cd terraform/environments/dev
terraform init
```

### 6. Review and Apply

```bash
terraform plan
terraform apply
```

## Building and Pushing Docker Images

### Quick Deploy (Recommended)

Use the provided deployment script:

```bash
# Deploy all services to dev environment
./scripts/deploy-to-ecr.sh dev

# Deploy specific service only
./scripts/deploy-to-ecr.sh dev backend
./scripts/deploy-to-ecr.sh dev frontend
./scripts/deploy-to-ecr.sh dev mpc-node

# Skip build and push existing images
./scripts/deploy-to-ecr.sh dev --skip-build
```

### Manual Deployment

If you prefer to build and push manually:

```bash
# Get ECR repository URLs from Terraform
cd terraform/environments/dev
export BACKEND_REPO=$(terraform output -raw backend_repository_url)
export FRONTEND_REPO=$(terraform output -raw frontend_repository_url)
export MPC_NODE_REPO=$(terraform output -raw mpc_node_repository_url)

# Login to ECR
aws ecr get-login-password --region ap-northeast-1 | \
  docker login --username AWS --password-stdin 719037119908.dkr.ecr.ap-northeast-1.amazonaws.com

# Build and push backend
docker build -f packages/server/Dockerfile -t ${BACKEND_REPO}:latest .
docker push ${BACKEND_REPO}:latest

# Build and push frontend
docker build -f packages/nextjs/Dockerfile -t ${FRONTEND_REPO}:latest .
docker push ${FRONTEND_REPO}:latest

# Build and push MPC node
docker build -f packages/zk-mpc-node/Dockerfile -t ${MPC_NODE_REPO}:latest .
docker push ${MPC_NODE_REPO}:latest
```

### Update ECS Services

After pushing new images, update ECS services to use them:

```bash
# Update all services
./scripts/update-ecs-service.sh dev

# Update specific service
./scripts/update-ecs-service.sh dev backend
```

## Outputs

After deployment, Terraform will output:

```bash
terraform output
```

Key outputs:

- **alb_dns_name**: ALB endpoint for backend API access
- **backend_repository_url**: ECR repository URL for backend
- **frontend_repository_url**: ECR repository URL for frontend
- **mpc_node_repository_url**: ECR repository URL for MPC node
- **ecs_cluster_name**: ECS cluster name
- **backend_service_name**: Backend ECS service name
- **vpc_id**: VPC ID
- **sops_kms_key_arn**: KMS key ARN for SOPS encryption

## Cost Optimization

### Dev Environment

- **Fargate Spot**: 50% of capacity (~50-70% cost savings on compute)
- **No NAT Gateway**: ~$32/month savings (uses public subnets with public IPs)
- **CloudWatch logs retention**: 7 days
- **Smaller CPU/Memory**: 512 CPU / 1024 MB for backend

**Estimated monthly cost (Dev)**: ~$70-120

### Production Recommendations

- Enable NAT Gateway for private subnet deployment (security best practice)
- Increase log retention (30+ days)
- Enable ALB deletion protection
- Use ACM certificate for HTTPS
- Configure CloudWatch Alarms for monitoring
- Enable AWS Backup for critical resources

**Estimated monthly cost (Prod)**: ~$250-350

## Monitoring

- CloudWatch Logs: `/ecs/zk-werewolf-{env}`
- ECS Container Insights enabled
- Custom health checks for all services

## Cleanup

To destroy all resources:

```bash
terraform destroy
```
