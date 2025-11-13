# =============================================================================
# Variables
# =============================================================================

variable "name_prefix" {
  description = "Prefix for ECR repository names"
  type        = string
}

variable "backend_enabled" {
  description = "Enable backend ECR repository"
  type        = bool
  default     = true
}

variable "frontend_enabled" {
  description = "Enable frontend ECR repository"
  type        = bool
  default     = true
}

variable "mpc_node_enabled" {
  description = "Enable MPC node ECR repository"
  type        = bool
  default     = true
}

variable "blockchain_enabled" {
  description = "Enable blockchain ECR repository"
  type        = bool
  default     = true
}

variable "image_tag_mutability" {
  description = "Image tag mutability setting (MUTABLE or IMMUTABLE)"
  type        = string
  default     = "MUTABLE"

  validation {
    condition     = contains(["MUTABLE", "IMMUTABLE"], var.image_tag_mutability)
    error_message = "image_tag_mutability must be either MUTABLE or IMMUTABLE"
  }
}

variable "image_count_limit" {
  description = "Number of images to keep in ECR repositories"
  type        = number
  default     = 10
}

variable "force_delete" {
  description = "Force delete ECR repositories even if they contain images"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Additional tags for ECR repositories"
  type        = map(string)
  default     = {}
}

# =============================================================================
# Resources
# =============================================================================

locals {
  repositories = {
    backend    = var.backend_enabled ? 1 : 0
    frontend   = var.frontend_enabled ? 1 : 0
    mpc_node   = var.mpc_node_enabled ? 1 : 0
    blockchain = var.blockchain_enabled ? 1 : 0
  }
}

# Backend ECR Repository
module "ecr_backend" {
  source  = "terraform-aws-modules/ecr/aws"
  version = "~> 2.0"

  count = local.repositories.backend

  repository_name                 = "${var.name_prefix}-backend"
  repository_image_tag_mutability = var.image_tag_mutability

  repository_lifecycle_policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last ${var.image_count_limit} images"
        selection = {
          tagStatus     = "any"
          countType     = "imageCountMoreThan"
          countNumber   = var.image_count_limit
        }
        action = {
          type = "expire"
        }
      }
    ]
  })

  repository_force_delete = var.force_delete

  tags = merge(
    var.tags,
    {
      Name    = "${var.name_prefix}-backend"
      Service = "backend"
    }
  )
}

# Frontend ECR Repository
module "ecr_frontend" {
  source  = "terraform-aws-modules/ecr/aws"
  version = "~> 2.0"

  count = local.repositories.frontend

  repository_name                 = "${var.name_prefix}-frontend"
  repository_image_tag_mutability = var.image_tag_mutability

  repository_lifecycle_policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last ${var.image_count_limit} images"
        selection = {
          tagStatus     = "any"
          countType     = "imageCountMoreThan"
          countNumber   = var.image_count_limit
        }
        action = {
          type = "expire"
        }
      }
    ]
  })

  repository_force_delete = var.force_delete

  tags = merge(
    var.tags,
    {
      Name    = "${var.name_prefix}-frontend"
      Service = "frontend"
    }
  )
}

# MPC Node ECR Repository
module "ecr_mpc_node" {
  source  = "terraform-aws-modules/ecr/aws"
  version = "~> 2.0"

  count = local.repositories.mpc_node

  repository_name                 = "${var.name_prefix}-mpc-node"
  repository_image_tag_mutability = var.image_tag_mutability

  repository_lifecycle_policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last ${var.image_count_limit} images"
        selection = {
          tagStatus     = "any"
          countType     = "imageCountMoreThan"
          countNumber   = var.image_count_limit
        }
        action = {
          type = "expire"
        }
      }
    ]
  })

  repository_force_delete = var.force_delete

  tags = merge(
    var.tags,
    {
      Name    = "${var.name_prefix}-mpc-node"
      Service = "mpc-node"
    }
  )
}

# Blockchain ECR Repository
module "ecr_blockchain" {
  source  = "terraform-aws-modules/ecr/aws"
  version = "~> 2.0"

  count = local.repositories.blockchain

  repository_name                 = "${var.name_prefix}-blockchain"
  repository_image_tag_mutability = var.image_tag_mutability

  repository_lifecycle_policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last ${var.image_count_limit} images"
        selection = {
          tagStatus     = "any"
          countType     = "imageCountMoreThan"
          countNumber   = var.image_count_limit
        }
        action = {
          type = "expire"
        }
      }
    ]
  })

  repository_force_delete = var.force_delete

  tags = merge(
    var.tags,
    {
      Name    = "${var.name_prefix}-blockchain"
      Service = "blockchain"
    }
  )
}

# =============================================================================
# Outputs
# =============================================================================

output "backend_repository_url" {
  description = "The URL of the backend ECR repository"
  value       = var.backend_enabled ? module.ecr_backend[0].repository_url : null
}

output "backend_repository_arn" {
  description = "The ARN of the backend ECR repository"
  value       = var.backend_enabled ? module.ecr_backend[0].repository_arn : null
}

output "frontend_repository_url" {
  description = "The URL of the frontend ECR repository"
  value       = var.frontend_enabled ? module.ecr_frontend[0].repository_url : null
}

output "frontend_repository_arn" {
  description = "The ARN of the frontend ECR repository"
  value       = var.frontend_enabled ? module.ecr_frontend[0].repository_arn : null
}

output "mpc_node_repository_url" {
  description = "The URL of the MPC node ECR repository"
  value       = var.mpc_node_enabled ? module.ecr_mpc_node[0].repository_url : null
}

output "mpc_node_repository_arn" {
  description = "The ARN of the MPC node ECR repository"
  value       = var.mpc_node_enabled ? module.ecr_mpc_node[0].repository_arn : null
}

output "blockchain_repository_url" {
  description = "The URL of the blockchain ECR repository"
  value       = var.blockchain_enabled ? module.ecr_blockchain[0].repository_url : null
}

output "blockchain_repository_arn" {
  description = "The ARN of the blockchain ECR repository"
  value       = var.blockchain_enabled ? module.ecr_blockchain[0].repository_arn : null
}

output "repository_urls" {
  description = "Map of all repository URLs"
  value = {
    backend    = var.backend_enabled ? module.ecr_backend[0].repository_url : null
    frontend   = var.frontend_enabled ? module.ecr_frontend[0].repository_url : null
    mpc_node   = var.mpc_node_enabled ? module.ecr_mpc_node[0].repository_url : null
    blockchain = var.blockchain_enabled ? module.ecr_blockchain[0].repository_url : null
  }
}
