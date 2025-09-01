provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "zk-werewolf"
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}

locals {
  name = "${var.project_name}-${var.environment}"
  
  azs = slice(data.aws_availability_zones.available.names, 0, 2)
}