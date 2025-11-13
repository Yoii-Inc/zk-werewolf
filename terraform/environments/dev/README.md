# ZK Werewolf - Dev Environment

Development environment infrastructure for ZK Werewolf project.

## Prerequisites

- Terraform >= 1.2.0
- AWS CLI configured with appropriate credentials
- Terraform Cloud account (for remote state)

## Infrastructure Components

This configuration creates:

- **SOPS KMS Key**: For encrypting secrets
- **VPC**: Network infrastructure with public and private subnets
- **ECR Repositories**: Container registries for:
  - Backend API
  - Frontend
  - MPC Nodes
  - Blockchain (Anvil)
- **ECS Cluster**: Container orchestration cluster

## Setup

1. Copy the example tfvars file:
   ```bash
   cp terraform.tfvars.example terraform.tfvars
   ```

2. Edit `terraform.tfvars` with your AWS account ID:
   ```hcl
   AWS_ACCOUNT_ID = "your-aws-account-id"
   ```

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

## Outputs

After applying, you'll get:

- VPC ID and subnet IDs
- ECR repository URLs for all services
- ECS cluster details
- IAM role ARNs for ECS tasks
- SOPS KMS key ARN

## Cost Optimization

This dev environment uses:

- Single NAT Gateway (instead of Multi-AZ)
- VPC Flow Logs disabled
- 7-day log retention
- Regular Fargate (not Spot) for stability

Estimated monthly cost: ~$50-100

## Terraform Cloud

This project uses Terraform Cloud for remote state management.

Workspace: `zk-werewolf-dev`
Organization: `Yoii`

## Cleanup

To destroy all resources:

```bash
terraform destroy
```

Note: ECR repositories have `force_delete = true` in dev, so they will be deleted even if they contain images.
