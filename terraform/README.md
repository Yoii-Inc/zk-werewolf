# ZK Werewolf Terraform Infrastructure

This Terraform configuration deploys the ZK Werewolf application to AWS ECS using Fargate and various AWS services.

## Architecture

- **Frontend**: CloudFront + S3 for static hosting
- **Backend API**: ECS Fargate behind Application Load Balancer
- **MPC Nodes**: 3 ECS Fargate services with service discovery
- **Blockchain**: ECS Fargate service running Anvil
- **Database**: Supabase Cloud (external)
- **Networking**: VPC with public/private subnets across 2 AZs

## Prerequisites

1. AWS CLI configured with appropriate credentials
2. Terraform >= 1.0
3. Docker images pushed to ECR repositories
4. Supabase project created

## Setup

1. Copy the example variables file:
   ```bash
   cp terraform.tfvars.example terraform.tfvars
   ```

2. Edit `terraform.tfvars` with your configuration:
   - Update Supabase connection strings
   - Set JWT secret
   - Configure domain name and certificate (optional)

3. Initialize Terraform:
   ```bash
   terraform init
   ```

4. Review the plan:
   ```bash
   terraform plan
   ```

5. Apply the configuration:
   ```bash
   terraform apply
   ```

## Building and Pushing Docker Images

Before deploying, you need to build and push Docker images to ECR:

```bash
# Login to ECR
aws ecr get-login-password --region ap-northeast-1 | docker login --username AWS --password-stdin <account-id>.dkr.ecr.ap-northeast-1.amazonaws.com

# Build and push backend
docker build -f packages/server/Dockerfile -t <backend-ecr-url>:latest .
docker push <backend-ecr-url>:latest

# Build and push MPC node
docker build -f packages/zk-mpc-node/Dockerfile -t <mpc-node-ecr-url>:latest .
docker push <mpc-node-ecr-url>:latest

# Build and push blockchain
docker build -f packages/foundry/Dockerfile -t <blockchain-ecr-url>:latest .
docker push <blockchain-ecr-url>:latest

# Build and push frontend (for S3 deployment)
cd packages/nextjs
npm run build
aws s3 sync out/ s3://<frontend-bucket-name>/ --delete
```

## Outputs

After deployment, Terraform will output:
- ALB DNS name for API access
- ECR repository URLs
- CloudFront distribution URL for frontend
- ECS cluster and service names

## Cost Optimization

- Uses Fargate Spot for 50% of capacity
- CloudWatch logs retention set to 7 days
- Consider using smaller instance sizes for development

## Monitoring

- CloudWatch Logs: `/ecs/zk-werewolf-{env}`
- ECS Container Insights enabled
- Custom health checks for all services

## Cleanup

To destroy all resources:
```bash
terraform destroy
```