# SOPS KMS Outputs
output "sops_kms_key_id" {
  description = "SOPS KMS key ID"
  value       = module.sops_kms.key_id
}

output "sops_kms_key_arn" {
  description = "SOPS KMS key ARN"
  value       = module.sops_kms.key_arn
}

# VPC Outputs
output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "vpc_cidr_block" {
  description = "VPC CIDR block"
  value       = module.vpc.vpc_cidr_block
}

output "private_subnets" {
  description = "Private subnet IDs"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "Public subnet IDs"
  value       = module.vpc.public_subnets
}

output "nat_public_ips" {
  description = "NAT Gateway public IPs"
  value       = module.vpc.nat_public_ips
}

# ECR Outputs
output "ecr_repository_urls" {
  description = "ECR repository URLs"
  value       = module.ecr.repository_urls
}

output "backend_repository_url" {
  description = "Backend ECR repository URL"
  value       = module.ecr.backend_repository_url
}

output "frontend_repository_url" {
  description = "Frontend ECR repository URL"
  value       = module.ecr.frontend_repository_url
}

output "mpc_node_repository_url" {
  description = "MPC node ECR repository URL"
  value       = module.ecr.mpc_node_repository_url
}

output "blockchain_repository_url" {
  description = "Blockchain ECR repository URL"
  value       = module.ecr.blockchain_repository_url
}

# ECS Cluster Outputs
output "ecs_cluster_id" {
  description = "ECS cluster ID"
  value       = module.ecs_cluster.cluster_id
}

output "ecs_cluster_arn" {
  description = "ECS cluster ARN"
  value       = module.ecs_cluster.cluster_arn
}

output "ecs_cluster_name" {
  description = "ECS cluster name"
  value       = module.ecs_cluster.cluster_name
}

output "ecs_task_execution_role_arn" {
  description = "ECS task execution role ARN"
  value       = module.ecs_cluster.task_execution_role_arn
}

output "ecs_task_role_arn" {
  description = "ECS task role ARN"
  value       = module.ecs_cluster.task_role_arn
}

output "ecs_cloudwatch_log_group_name" {
  description = "ECS CloudWatch log group name"
  value       = module.ecs_cluster.cloudwatch_log_group_name
}

# ALB Outputs
output "alb_dns_name" {
  description = "ALB DNS name"
  value       = module.alb.alb_dns_name
}

output "alb_arn" {
  description = "ALB ARN"
  value       = module.alb.alb_arn
}

# Backend Service Outputs
output "backend_service_name" {
  description = "Backend ECS service name"
  value       = module.backend_service.service_name
}

output "backend_task_definition_arn" {
  description = "Backend task definition ARN"
  value       = module.backend_service.task_definition_arn
}
