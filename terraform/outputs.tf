output "alb_dns_name" {
  description = "DNS name of the load balancer"
  value       = module.alb.dns_name
}

output "ecr_repository_urls" {
  description = "ECR repository URLs"
  value = {
    backend    = local.ecr_repositories["backend"]
    mpc_node   = local.ecr_repositories["mpc-node"]
    blockchain = local.ecr_repositories["blockchain"]
    frontend   = local.ecr_repositories["frontend"]
  }
}

output "ecs_cluster_name" {
  description = "Name of the ECS cluster"
  value       = module.ecs.cluster_name
}

output "ecs_cluster_id" {
  description = "ID of the ECS cluster"
  value       = module.ecs.cluster_id
}

output "vpc_id" {
  description = "ID of the VPC"
  value       = module.vpc.vpc_id
}

output "private_subnets" {
  description = "List of IDs of private subnets"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "List of IDs of public subnets"
  value       = module.vpc.public_subnets
}

output "backend_service_name" {
  description = "Name of the backend ECS service"
  value       = aws_ecs_service.backend.name
}

output "mpc_node_service_names" {
  description = "Names of the MPC node ECS services"
  value       = [for s in aws_ecs_service.mpc_node : s.name]
}

output "blockchain_service_name" {
  description = "Name of the blockchain ECS service"
  value       = aws_ecs_service.blockchain.name
}

output "service_discovery_namespace" {
  description = "Service discovery namespace for internal communication"
  value       = aws_service_discovery_private_dns_namespace.internal.name
}

output "cloudfront_distribution_url" {
  description = "CloudFront distribution URL for the frontend"
  value       = "https://${aws_cloudfront_distribution.frontend.domain_name}"
}

output "s3_bucket_name" {
  description = "Name of the S3 bucket for frontend hosting"
  value       = module.s3_bucket.s3_bucket_id
}