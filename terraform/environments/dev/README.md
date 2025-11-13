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

### 1. Configure AWS Credentials

```bash
# Login with AWS SSO
aws-sso-util login --profile yoii-crypto-dev.AWSAdministratorAccess
export AWS_PROFILE=yoii-crypto-dev.AWSAdministratorAccess
```

### 2. Configure Terraform Variables

```bash
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your AWS account ID
```

### 3. Configure Secrets with SOPS

```bash
# Install SOPS if not already installed
brew install sops  # macOS

# Copy the secrets template
cp secrets.enc.yaml.example secrets.enc.yaml

# Edit the secrets file (opens in your $EDITOR)
sops secrets.enc.yaml

# The file will be automatically encrypted when saved
```

**Important**: Never commit unencrypted secrets. The `.gitignore` is configured to only allow `secrets.enc.yaml` (encrypted) files.

### 4. Deploy Infrastructure

```bash
# Initialize Terraform
terraform init

# Review the plan
terraform plan

# Apply the configuration
terraform apply
```

## Managing Secrets with SOPS

### Viewing Encrypted Secrets

```bash
# View decrypted content
sops -d secrets.enc.yaml

# Edit secrets (automatically re-encrypts on save)
sops secrets.enc.yaml
```

### Using Secrets in Terraform

Secrets can be loaded using the `yamldecode` function:

```hcl
locals {
  secrets = yamldecode(file("${path.module}/secrets.enc.yaml"))
}

# Use in environment variables
environment_variables = [
  {
    name  = "SUPABASE_URL"
    value = local.secrets.backend.supabase_url
  }
]
```

### Rotating Secrets

1. Edit the secrets file: `sops secrets.enc.yaml`
2. Update values
3. Save (automatically re-encrypts)
4. Commit and push
5. Run `terraform apply` to update ECS tasks

## Outputs

After applying, you'll get:

- **Application URL**: Main application endpoint
- **API URL**: Backend API endpoint
- **WebSocket URL**: WebSocket endpoint
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
