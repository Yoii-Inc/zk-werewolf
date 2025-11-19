terraform {
  cloud {
    organization = "Yoii"
    workspaces {
      name = "zk-werewolf-dev"
    }
  }

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    sops = {
      source  = "carlpett/sops"
      version = "~> 1.0"
    }
  }
  required_version = ">= 1.2.0"
}


provider "aws" {
  region = var.AWS_DEFAULT_REGION
  allowed_account_ids = [
    var.AWS_ACCOUNT_ID
  ]

  default_tags {
    tags = {
      Project     = "zk-werewolf"
      Environment = "dev"
      ManagedBy   = "terraform"
    }
  }
}

locals {
  name = "zk-werewolf-dev"

  # Load secrets from SOPS encrypted file if it exists
  secrets_file_exists = fileexists("${path.module}/secrets.enc.yaml")
}

# SOPS data source (only if secrets file exists)
data "sops_file" "secrets" {
  count = local.secrets_file_exists ? 1 : 0
  source_file = "${path.module}/secrets.enc.yaml"
}

locals {
  # Parse secrets if file exists, otherwise use empty map
  secrets = local.secrets_file_exists ? yamldecode(data.sops_file.secrets[0].raw) : {
    backend = {
      supabase_url = ""
      supabase_key = ""
      jwt_secret   = ""
    }
    mpc_nodes = {
      node_0 = {
        private_key = ""
      }
      node_1 = {
        private_key = ""
      }
      node_2 = {
        private_key = ""
      }
    }
    database = {
      host     = ""
      port     = ""
      username = ""
      password = ""
      database = ""
    }
  }
}

# KMS key for SOPS encryption
module "sops_kms" {
  source = "../../modules/sops-kms"

  name        = "${local.name}-sops-key"
  description = "SOPS encryption key for ZK Werewolf Dev"
  environment = "dev"
}

# VPC
module "vpc" {
  source = "../../modules/vpc"

  name                 = "${local.name}-vpc"
  cidr                 = "10.0.0.0/16"
  private_subnet_cidrs = ["10.0.1.0/24", "10.0.2.0/24"]
  public_subnet_cidrs  = ["10.0.101.0/24", "10.0.102.0/24"]

  enable_nat_gateway = false # Disabled - use public subnets for dev
  single_nat_gateway = false
  enable_flow_log    = false # Disabled for dev to save costs
}

# ECR Repositories
module "ecr" {
  source = "../../modules/ecr"

  name_prefix = local.name

  backend_enabled    = true
  frontend_enabled   = true
  mpc_node_enabled   = true
  blockchain_enabled = false # Blockchain not currently used

  image_tag_mutability = "MUTABLE"
  image_count_limit    = 10
  force_delete         = true # Allow force delete in dev environment
}

# ECS Cluster
module "ecs_cluster" {
  source = "../../modules/ecs-cluster"

  name   = "${local.name}-cluster"
  vpc_id = module.vpc.vpc_id

  log_retention_days = 7

  # Use Fargate Spot for dev (50-70% cost savings)
  fargate_capacity_providers = {
    FARGATE = {
      default_capacity_provider_strategy = {
        weight = 50
        base   = 1
      }
    }
    FARGATE_SPOT = {
      default_capacity_provider_strategy = {
        weight = 50
      }
    }
  }
}

# Security Groups
module "security_groups" {
  source = "../../modules/security-groups"

  name_prefix = local.name
  vpc_id      = module.vpc.vpc_id
}

# Application Load Balancer
module "alb" {
  source = "../../modules/alb"

  name              = "${local.name}-alb"
  vpc_id            = module.vpc.vpc_id
  subnet_ids        = module.vpc.public_subnets
  security_group_id = module.security_groups.alb_security_group_id

  certificate_arn            = "" # Add ACM certificate ARN for HTTPS
  enable_deletion_protection = false
}

# Backend ECS Service
module "backend_service" {
  source = "../../modules/ecs-service"

  service_name       = "${local.name}-backend"
  cluster_id         = module.ecs_cluster.cluster_id
  container_image    = "${module.ecr.backend_repository_url}:latest"
  container_port     = 8080
  cpu                = "512"
  memory             = "1024"
  desired_count      = 1
  launch_type        = null # Use capacity provider strategy

  capacity_provider_strategy = [
    {
      capacity_provider = "FARGATE"
      weight            = 50
      base              = 1
    },
    {
      capacity_provider = "FARGATE_SPOT"
      weight            = 50
    }
  ]

  subnet_ids         = module.vpc.public_subnets # Using public subnets (no NAT Gateway)
  security_group_ids = [module.security_groups.ecs_tasks_security_group_id]
  assign_public_ip   = true # Required without NAT Gateway

  target_group_arn   = module.alb.backend_target_group_arn
  execution_role_arn = module.ecs_cluster.task_execution_role_arn
  task_role_arn      = module.ecs_cluster.task_role_arn
  log_group_name     = module.ecs_cluster.cloudwatch_log_group_name

  environment_variables = concat([
    {
      name  = "PORT"
      value = "8080"
    },
    {
      name  = "ENVIRONMENT"
      value = "dev"
    }
  ], local.secrets_file_exists ? [
    {
      name  = "SUPABASE_URL"
      value = local.secrets.backend.supabase_url
    },
    {
      name  = "SUPABASE_KEY"
      value = local.secrets.backend.supabase_key
    },
    {
      name  = "JWT_SECRET"
      value = local.secrets.backend.jwt_secret
    }
  ] : [])

  health_check_grace_period = 60
  enable_execute_command    = true
}

# =============================================================================
# Frontend ECS Service
# =============================================================================

module "frontend_service" {
  source = "../../modules/ecs-service"

  service_name       = "${local.name}-frontend"
  cluster_id         = module.ecs_cluster.cluster_id
  container_image    = "${module.ecr.frontend_repository_url}:latest"
  container_port     = 3000
  cpu                = "256"
  memory             = "512"
  desired_count      = 1
  launch_type        = null

  capacity_provider_strategy = [
    {
      capacity_provider = "FARGATE"
      weight            = 50
      base              = 1
    },
    {
      capacity_provider = "FARGATE_SPOT"
      weight            = 50
    }
  ]

  subnet_ids         = module.vpc.public_subnets
  security_group_ids = [module.security_groups.ecs_tasks_security_group_id]
  assign_public_ip   = true

  target_group_arn   = module.alb.frontend_target_group_arn
  execution_role_arn = module.ecs_cluster.task_execution_role_arn
  task_role_arn      = module.ecs_cluster.task_role_arn
  log_group_name     = module.ecs_cluster.cloudwatch_log_group_name

  environment_variables = [
    {
      name  = "PORT"
      value = "3000"
    },
    {
      name  = "NEXT_PUBLIC_API_URL"
      value = "http://${module.alb.alb_dns_name}/api"
    },
    {
      name  = "NEXT_PUBLIC_WS_URL"
      value = "ws://${module.alb.alb_dns_name}/ws"
    }
  ]

  health_check_grace_period = 60
  enable_execute_command    = true
}
