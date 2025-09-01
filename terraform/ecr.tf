module "ecr" {
  source  = "terraform-aws-modules/ecr/aws"
  version = "~> 2.0"

  repository_name                 = "${local.name}-backend"
  repository_image_tag_mutability = "MUTABLE"

  repository_lifecycle_policy = jsonencode({
    rules = [
      {
        rulePriority = 1,
        description  = "Keep last 10 images",
        selection = {
          tagStatus     = "tagged",
          tagPrefixList = ["v"],
          countType     = "imageCountMoreThan",
          countNumber   = 10
        },
        action = {
          type = "expire"
        }
      }
    ]
  })

  tags = {
    Name = "${local.name}-backend-ecr"
  }
}

resource "aws_ecr_repository" "mpc_node" {
  name                 = "${local.name}-mpc-node"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_repository" "blockchain" {
  name                 = "${local.name}-blockchain"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_repository" "frontend" {
  name                 = "${local.name}-frontend"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

locals {
  ecr_repositories = {
    backend    = module.ecr.repository_url
    mpc-node   = aws_ecr_repository.mpc_node.repository_url
    blockchain = aws_ecr_repository.blockchain.repository_url
    frontend   = aws_ecr_repository.frontend.repository_url
  }
}