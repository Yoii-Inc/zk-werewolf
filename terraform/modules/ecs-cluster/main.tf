# =============================================================================
# Variables
# =============================================================================

variable "name" {
  description = "Name of the ECS cluster"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID where ECS cluster will be deployed"
  type        = string
}

variable "log_retention_days" {
  description = "CloudWatch Logs retention period in days"
  type        = number
  default     = 7
}

variable "fargate_capacity_providers" {
  description = "Fargate capacity provider configuration"
  type = map(object({
    default_capacity_provider_strategy = object({
      weight = number
      base   = optional(number, 0)
    })
  }))
  default = {
    FARGATE = {
      default_capacity_provider_strategy = {
        weight = 100
        base   = 1
      }
    }
    FARGATE_SPOT = {
      default_capacity_provider_strategy = {
        weight = 0
      }
    }
  }
}

variable "tags" {
  description = "Additional tags for ECS resources"
  type        = map(string)
  default     = {}
}

# =============================================================================
# Resources
# =============================================================================

module "ecs" {
  source  = "terraform-aws-modules/ecs/aws"
  version = "~> 5.0"

  cluster_name = var.name

  # CloudWatch Container Insights
  cluster_configuration = {
    execute_command_configuration = {
      logging = "OVERRIDE"
      log_configuration = {
        cloud_watch_log_group_name = aws_cloudwatch_log_group.ecs.name
      }
    }
  }

  # Fargate capacity providers
  fargate_capacity_providers = var.fargate_capacity_providers

  tags = var.tags
}

# CloudWatch Log Group for ECS
resource "aws_cloudwatch_log_group" "ecs" {
  name              = "/ecs/${var.name}"
  retention_in_days = var.log_retention_days

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-ecs-logs"
    }
  )
}

# ECS Task Execution Role
data "aws_iam_policy_document" "ecs_task_execution_role" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "ecs_task_execution_role" {
  name               = "${var.name}-ecs-task-execution-role"
  assume_role_policy = data.aws_iam_policy_document.ecs_task_execution_role.json

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-ecs-task-execution-role"
    }
  )
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution_role_policy" {
  role       = aws_iam_role.ecs_task_execution_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Additional policy for ECR access
resource "aws_iam_role_policy" "ecs_task_execution_role_ecr" {
  name = "${var.name}-ecs-task-execution-ecr"
  role = aws_iam_role.ecs_task_execution_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken",
          "ecr:BatchCheckLayerAvailability",
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage"
        ]
        Resource = "*"
      }
    ]
  })
}

# Additional policy for CloudWatch Logs
resource "aws_iam_role_policy" "ecs_task_execution_role_logs" {
  name = "${var.name}-ecs-task-execution-logs"
  role = aws_iam_role.ecs_task_execution_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "${aws_cloudwatch_log_group.ecs.arn}:*"
      }
    ]
  })
}

# ECS Task Role (for application containers)
resource "aws_iam_role" "ecs_task_role" {
  name = "${var.name}-ecs-task-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-ecs-task-role"
    }
  )
}

# =============================================================================
# Outputs
# =============================================================================

output "cluster_id" {
  description = "The ID of the ECS cluster"
  value       = module.ecs.cluster_id
}

output "cluster_arn" {
  description = "The ARN of the ECS cluster"
  value       = module.ecs.cluster_arn
}

output "cluster_name" {
  description = "The name of the ECS cluster"
  value       = module.ecs.cluster_name
}

output "task_execution_role_arn" {
  description = "The ARN of the ECS task execution role"
  value       = aws_iam_role.ecs_task_execution_role.arn
}

output "task_execution_role_name" {
  description = "The name of the ECS task execution role"
  value       = aws_iam_role.ecs_task_execution_role.name
}

output "task_role_arn" {
  description = "The ARN of the ECS task role"
  value       = aws_iam_role.ecs_task_role.arn
}

output "task_role_name" {
  description = "The name of the ECS task role"
  value       = aws_iam_role.ecs_task_role.name
}

output "cloudwatch_log_group_name" {
  description = "The name of the CloudWatch log group for ECS"
  value       = aws_cloudwatch_log_group.ecs.name
}

output "cloudwatch_log_group_arn" {
  description = "The ARN of the CloudWatch log group for ECS"
  value       = aws_cloudwatch_log_group.ecs.arn
}
