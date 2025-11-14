# =============================================================================
# Variables
# =============================================================================

variable "name" {
  description = "Name of the KMS key"
  type        = string
}

variable "description" {
  description = "Description of the KMS key"
  type        = string
}

variable "environment" {
  description = "Environment name (e.g., dev, staging, prod)"
  type        = string
}

variable "deletion_window_in_days" {
  description = "Duration in days after which the key is deleted after destruction"
  type        = number
  default     = 30
}

variable "tags" {
  description = "Additional tags for the KMS key"
  type        = map(string)
  default     = {}
}

# =============================================================================
# Resources
# =============================================================================

resource "aws_kms_key" "sops" {
  description             = var.description
  deletion_window_in_days = var.deletion_window_in_days
  enable_key_rotation     = true

  tags = merge(
    {
      Name        = var.name
      Environment = var.environment
      Purpose     = "SOPS encryption"
    },
    var.tags
  )
}

resource "aws_kms_alias" "sops" {
  name          = "alias/${var.name}"
  target_key_id = aws_kms_key.sops.key_id
}

data "aws_caller_identity" "current" {}
data "aws_region" "current" {}

resource "aws_kms_key_policy" "sops" {
  key_id = aws_kms_key.sops.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow use of the key for SOPS"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:ReEncrypt*",
          "kms:GenerateDataKey*",
          "kms:DescribeKey"
        ]
        Resource = "*"
      }
    ]
  })
}

# =============================================================================
# Outputs
# =============================================================================

output "key_id" {
  description = "The KMS key ID"
  value       = aws_kms_key.sops.key_id
}

output "key_arn" {
  description = "The KMS key ARN"
  value       = aws_kms_key.sops.arn
}

output "alias_arn" {
  description = "The KMS alias ARN"
  value       = aws_kms_alias.sops.arn
}

output "alias_name" {
  description = "The KMS alias name"
  value       = aws_kms_alias.sops.name
}
