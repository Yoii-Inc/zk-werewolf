# =============================================================================
# Variables
# =============================================================================

variable "name" {
  description = "Name for the service discovery namespace"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID where the namespace will be created"
  type        = string
}

variable "namespace_name" {
  description = "DNS namespace name (e.g., 'mpc.local')"
  type        = string
  default     = "mpc.local"
}

variable "services" {
  description = "Map of service names to create"
  type = map(object({
    dns_ttl = optional(number, 10)
  }))
  default = {}
}

variable "tags" {
  description = "Additional tags"
  type        = map(string)
  default     = {}
}

# =============================================================================
# Resources
# =============================================================================

resource "aws_service_discovery_private_dns_namespace" "this" {
  name        = var.namespace_name
  description = "Private DNS namespace for ${var.name}"
  vpc         = var.vpc_id

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-namespace"
    }
  )
}

resource "aws_service_discovery_service" "this" {
  for_each = var.services

  name = each.key

  dns_config {
    namespace_id = aws_service_discovery_private_dns_namespace.this.id

    dns_records {
      ttl  = each.value.dns_ttl
      type = "A"
    }

    routing_policy = "MULTIVALUE"
  }

  health_check_custom_config {
    failure_threshold = 1
  }

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-${each.key}"
    }
  )
}

# =============================================================================
# Outputs
# =============================================================================

output "namespace_id" {
  description = "The ID of the service discovery namespace"
  value       = aws_service_discovery_private_dns_namespace.this.id
}

output "namespace_name" {
  description = "The name of the service discovery namespace"
  value       = aws_service_discovery_private_dns_namespace.this.name
}

output "namespace_arn" {
  description = "The ARN of the service discovery namespace"
  value       = aws_service_discovery_private_dns_namespace.this.arn
}

output "service_arns" {
  description = "Map of service names to their ARNs"
  value       = { for k, v in aws_service_discovery_service.this : k => v.arn }
}

output "service_ids" {
  description = "Map of service names to their IDs"
  value       = { for k, v in aws_service_discovery_service.this : k => v.id }
}
