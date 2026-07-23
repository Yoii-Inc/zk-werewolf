# =============================================================================
# Variables
# =============================================================================

variable "name" {
  description = "Name of the Application Load Balancer"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID where ALB will be created"
  type        = string
}

variable "subnet_ids" {
  description = "List of subnet IDs for the ALB"
  type        = list(string)
}

variable "security_group_id" {
  description = "Security group ID for the ALB"
  type        = string
}

variable "certificate_arn" {
  description = "ARN of ACM certificate for HTTPS (optional)"
  type        = string
  default     = ""
}

variable "enable_deletion_protection" {
  description = "Enable deletion protection for ALB"
  type        = bool
  default     = false
}

variable "enable_access_logs" {
  description = "Enable ALB access logs delivered to an S3 bucket"
  type        = bool
  default     = true
}

variable "access_logs_prefix" {
  description = "Prefix for ALB access log objects in the S3 bucket"
  type        = string
  default     = "alb"
}

variable "access_logs_retention_days" {
  description = "Number of days to retain ALB access logs before they expire"
  type        = number
  default     = 30
}

variable "access_logs_force_destroy" {
  description = "Allow Terraform to destroy the access logs bucket even if it still contains objects"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Additional tags for ALB resources"
  type        = map(string)
  default     = {}
}

# =============================================================================
# Data Sources
# =============================================================================

data "aws_caller_identity" "current" {}

data "aws_region" "current" {}

# =============================================================================
# Resources
# =============================================================================

# S3 bucket for ALB access logs
resource "aws_s3_bucket" "access_logs" {
  count = var.enable_access_logs ? 1 : 0

  bucket        = "${var.name}-access-logs-${data.aws_caller_identity.current.account_id}"
  force_destroy = var.access_logs_force_destroy

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-access-logs"
    }
  )
}

resource "aws_s3_bucket_public_access_block" "access_logs" {
  count = var.enable_access_logs ? 1 : 0

  bucket = aws_s3_bucket.access_logs[0].id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "access_logs" {
  count = var.enable_access_logs ? 1 : 0

  bucket = aws_s3_bucket.access_logs[0].id

  rule {
    id     = "expire-access-logs"
    status = "Enabled"

    filter {}

    expiration {
      days = var.access_logs_retention_days
    }
  }
}

# Grants the regional ELB log delivery service permission to write access
# logs. Uses the log delivery service principal (works in all regions,
# including those launched after August 2022) rather than the legacy
# per-region ELB account ID.
data "aws_iam_policy_document" "access_logs" {
  count = var.enable_access_logs ? 1 : 0

  statement {
    sid     = "AWSLogDeliveryWrite"
    effect  = "Allow"
    actions = ["s3:PutObject"]

    principals {
      type        = "Service"
      identifiers = ["logdelivery.elasticloadbalancing.amazonaws.com"]
    }

    resources = [
      "${aws_s3_bucket.access_logs[0].arn}/${var.access_logs_prefix}/AWSLogs/${data.aws_caller_identity.current.account_id}/*"
    ]

    condition {
      test     = "ArnLike"
      variable = "aws:SourceArn"
      values   = ["arn:aws:elasticloadbalancing:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:loadbalancer/*"]
    }
  }
}

resource "aws_s3_bucket_policy" "access_logs" {
  count = var.enable_access_logs ? 1 : 0

  bucket = aws_s3_bucket.access_logs[0].id
  policy = data.aws_iam_policy_document.access_logs[0].json
}

resource "aws_lb" "main" {
  name               = var.name
  internal           = false
  load_balancer_type = "application"
  security_groups    = [var.security_group_id]
  subnets            = var.subnet_ids

  enable_deletion_protection       = var.enable_deletion_protection
  enable_http2                     = true
  enable_cross_zone_load_balancing = true

  dynamic "access_logs" {
    for_each = var.enable_access_logs ? [1] : []
    content {
      bucket  = aws_s3_bucket.access_logs[0].id
      prefix  = var.access_logs_prefix
      enabled = true
    }
  }

  # The bucket policy must exist before ELB validates write access.
  depends_on = [aws_s3_bucket_policy.access_logs]

  tags = merge(
    var.tags,
    {
      Name = var.name
    }
  )
}

# Target Group for Frontend
resource "aws_lb_target_group" "frontend" {
  name        = "${var.name}-frontend"
  port        = 3000
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    interval            = 30
    matcher             = "200"
    path                = "/"
    port                = "traffic-port"
    protocol            = "HTTP"
    timeout             = 5
    unhealthy_threshold = 3
  }

  deregistration_delay = 30

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-frontend-tg"
    }
  )
}

# HTTP Listener (redirect to HTTPS if certificate is provided, otherwise forward to frontend)
resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.main.arn
  port              = "80"
  protocol          = "HTTP"

  default_action {
    type = var.certificate_arn != "" ? "redirect" : "forward"

    dynamic "redirect" {
      for_each = var.certificate_arn != "" ? [1] : []
      content {
        port        = "443"
        protocol    = "HTTPS"
        status_code = "HTTP_301"
      }
    }

    target_group_arn = var.certificate_arn == "" ? aws_lb_target_group.frontend.arn : null
  }
}

# HTTPS Listener (optional, if certificate provided, forward to frontend by default)
resource "aws_lb_listener" "https" {
  count = var.certificate_arn != "" ? 1 : 0

  load_balancer_arn = aws_lb.main.arn
  port              = "443"
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.frontend.arn
  }
}

# Target Group for Backend
resource "aws_lb_target_group" "backend" {
  name        = "${var.name}-backend"
  port        = 8080
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    interval            = 30
    matcher             = "200"
    path                = "/health"
    port                = "traffic-port"
    protocol            = "HTTP"
    timeout             = 5
    unhealthy_threshold = 3
  }

  deregistration_delay = 30

  tags = merge(
    var.tags,
    {
      Name = "${var.name}-backend-tg"
    }
  )
}

# Listener Rule for Backend
resource "aws_lb_listener_rule" "backend" {
  listener_arn = var.certificate_arn != "" ? aws_lb_listener.https[0].arn : aws_lb_listener.http.arn
  priority     = 100

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.backend.arn
  }

  condition {
    path_pattern {
      values = ["/api", "/api/*", "/ws", "/ws/*"]
    }
  }
}

# =============================================================================
# Outputs
# =============================================================================

output "alb_arn" {
  description = "The ARN of the Application Load Balancer"
  value       = aws_lb.main.arn
}

output "alb_dns_name" {
  description = "The DNS name of the Application Load Balancer"
  value       = aws_lb.main.dns_name
}

output "alb_zone_id" {
  description = "The zone ID of the Application Load Balancer"
  value       = aws_lb.main.zone_id
}

output "access_logs_bucket_name" {
  description = "The name of the S3 bucket storing ALB access logs (null if disabled)"
  value       = var.enable_access_logs ? aws_s3_bucket.access_logs[0].id : null
}

output "access_logs_bucket_arn" {
  description = "The ARN of the S3 bucket storing ALB access logs (null if disabled)"
  value       = var.enable_access_logs ? aws_s3_bucket.access_logs[0].arn : null
}

output "frontend_target_group_arn" {
  description = "The ARN of the frontend target group"
  value       = aws_lb_target_group.frontend.arn
}

output "backend_target_group_arn" {
  description = "The ARN of the backend target group"
  value       = aws_lb_target_group.backend.arn
}

output "http_listener_arn" {
  description = "The ARN of the HTTP listener"
  value       = aws_lb_listener.http.arn
}

output "https_listener_arn" {
  description = "The ARN of the HTTPS listener (if certificate provided)"
  value       = var.certificate_arn != "" ? aws_lb_listener.https[0].arn : null
}
