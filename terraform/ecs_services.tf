resource "aws_ecs_task_definition" "backend" {
  family                   = "${local.name}-backend"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "512"
  memory                   = "1024"
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn

  container_definitions = jsonencode([
    {
      name  = "backend"
      image = "${local.ecr_repositories["backend"]}:latest"
      
      portMappings = [
        {
          containerPort = 8080
          protocol      = "tcp"
        }
      ]
      
      environment = [
        {
          name  = "ZK_MPC_NODE_1"
          value = "http://mpc-node-0.werewolf.local:9000"
        },
        {
          name  = "ZK_MPC_NODE_2"
          value = "http://mpc-node-1.werewolf.local:9001"
        },
        {
          name  = "ZK_MPC_NODE_3"
          value = "http://mpc-node-2.werewolf.local:9002"
        },
        {
          name  = "RUST_LOG"
          value = "debug"
        },
        {
          name  = "DATABASE_URL"
          value = var.supabase_database_url
        },
        {
          name  = "REDIS_URL"
          value = var.supabase_redis_url
        },
        {
          name  = "JWT_SECRET"
          value = var.jwt_secret
        }
      ]
      
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "backend"
        }
      }
    }
  ])
}

resource "aws_ecs_service" "backend" {
  name            = "${local.name}-backend"
  cluster         = module.ecs.cluster_id
  task_definition = aws_ecs_task_definition.backend.arn
  desired_count   = var.backend_service_count

  launch_type = "FARGATE"

  network_configuration {
    subnets          = module.vpc.private_subnets
    security_groups  = [aws_security_group.backend.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = module.alb.target_groups["backend"].arn
    container_name   = "backend"
    container_port   = 8080
  }

  service_registries {
    registry_arn = aws_service_discovery_service.backend.arn
  }

  depends_on = [module.alb]
}

resource "aws_service_discovery_service" "backend" {
  name = "backend"

  dns_config {
    namespace_id = aws_service_discovery_private_dns_namespace.internal.id

    dns_records {
      ttl  = 10
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_security_group" "backend" {
  name        = "${local.name}-backend-sg"
  description = "Security group for backend service"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [module.alb.security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_ecs_task_definition" "mpc_node" {
  count = 3

  family                   = "${local.name}-mpc-node-${count.index}"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "1024"
  memory                   = "2048"
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn

  container_definitions = jsonencode([
    {
      name  = "mpc-node-${count.index}"
      image = "${local.ecr_repositories["mpc-node"]}:latest"
      
      command = ["start", "--id", "${count.index}", "--input", "./address/3"]
      
      portMappings = [
        {
          containerPort = 9000 + count.index
          protocol      = "tcp"
        },
        {
          containerPort = 8000 + count.index
          protocol      = "tcp"
        }
      ]
      
      environment = [
        {
          name  = "RUST_LOG"
          value = "debug"
        },
        {
          name  = "NODE_ID"
          value = tostring(count.index)
        }
      ]
      
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "mpc-node-${count.index}"
        }
      }
    }
  ])
}

resource "aws_ecs_service" "mpc_node" {
  count = 3

  name            = "${local.name}-mpc-node-${count.index}"
  cluster         = module.ecs.cluster_id
  task_definition = aws_ecs_task_definition.mpc_node[count.index].arn
  desired_count   = 1

  launch_type = "FARGATE"

  network_configuration {
    subnets          = module.vpc.private_subnets
    security_groups  = [aws_security_group.mpc_nodes.id]
    assign_public_ip = false
  }

  service_registries {
    registry_arn = aws_service_discovery_service.mpc_nodes[count.index].arn
  }
}

resource "aws_service_discovery_service" "mpc_nodes" {
  count = 3

  name = "mpc-node-${count.index}"

  dns_config {
    namespace_id = aws_service_discovery_private_dns_namespace.internal.id

    dns_records {
      ttl  = 10
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_security_group" "mpc_nodes" {
  name        = "${local.name}-mpc-nodes-sg"
  description = "Security group for MPC nodes"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port = 9000
    to_port   = 9002
    protocol  = "tcp"
    self      = true
  }

  ingress {
    from_port       = 9000
    to_port         = 9002
    protocol        = "tcp"
    security_groups = [aws_security_group.backend.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_ecs_task_definition" "blockchain" {
  family                   = "${local.name}-blockchain"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "512"
  memory                   = "1024"
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn

  container_definitions = jsonencode([
    {
      name  = "blockchain"
      image = "${local.ecr_repositories["blockchain"]}:latest"
      
      portMappings = [
        {
          containerPort = 8545
          protocol      = "tcp"
        }
      ]
      
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "blockchain"
        }
      }
    }
  ])
}

resource "aws_ecs_service" "blockchain" {
  name            = "${local.name}-blockchain"
  cluster         = module.ecs.cluster_id
  task_definition = aws_ecs_task_definition.blockchain.arn
  desired_count   = 1

  launch_type = "FARGATE"

  network_configuration {
    subnets          = module.vpc.private_subnets
    security_groups  = [aws_security_group.blockchain.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = module.alb.target_groups["blockchain"].arn
    container_name   = "blockchain"
    container_port   = 8545
  }

  service_registries {
    registry_arn = aws_service_discovery_service.blockchain.arn
  }

  depends_on = [module.alb]
}

resource "aws_service_discovery_service" "blockchain" {
  name = "blockchain"

  dns_config {
    namespace_id = aws_service_discovery_private_dns_namespace.internal.id

    dns_records {
      ttl  = 10
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_security_group" "blockchain" {
  name        = "${local.name}-blockchain-sg"
  description = "Security group for blockchain service"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 8545
    to_port         = 8545
    protocol        = "tcp"
    security_groups = [module.alb.security_group_id, aws_security_group.backend.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}