module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = local.name
  cidr = var.vpc_cidr

  azs             = data.aws_availability_zones.available.names
  private_subnets = var.private_subnet_cidrs
  public_subnets  = var.public_subnet_cidrs

  enable_nat_gateway = true
  enable_vpn_gateway = false
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name = "${local.name}-vpc"
  }
}

data "aws_availability_zones" "available" {
  state = "available"
}

resource "aws_service_discovery_private_dns_namespace" "internal" {
  name        = "werewolf.local"
  description = "Private DNS namespace for service discovery"
  vpc         = module.vpc.vpc_id
}