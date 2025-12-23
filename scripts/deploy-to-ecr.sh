#!/bin/bash
set -e

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TERRAFORM_DIR="${PROJECT_ROOT}/terraform/environments"

# Source common functions
source "${SCRIPT_DIR}/common.sh"

# Usage function
usage() {
    cat << EOF
Usage: $0 <environment> [service] [options]

Arguments:
  environment    Environment name (dev, staging, prod)
  service        Service to deploy (backend, frontend, mpc-node, all) [default: all]

Options:
  --skip-build   Skip Docker build, only push existing images
  --help         Show this help message

Examples:
  $0 dev                    # Deploy all services to dev
  $0 dev backend            # Deploy only backend to dev
  $0 dev --skip-build       # Push existing images without rebuilding
EOF
    exit 1
}

# Parse arguments
ENVIRONMENT=""
SERVICE="all"
SKIP_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --help)
            usage
            ;;
        *)
            if [ -z "$ENVIRONMENT" ]; then
                ENVIRONMENT=$1
            elif [ -z "$SERVICE" ] || [ "$SERVICE" = "all" ]; then
                SERVICE=$1
            else
                log_error "Unknown argument: $1"
            fi
            shift
            ;;
    esac
done

# Validate arguments
if [ -z "$ENVIRONMENT" ]; then
    log_error "Environment is required"
fi

validate_environment "${TERRAFORM_DIR}" "${ENVIRONMENT}"

# Check if Terraform is initialized
if [ ! -d "${TERRAFORM_DIR}/${ENVIRONMENT}/.terraform" ]; then
    log_warn "Terraform not initialized. Running terraform init..."
    cd "${TERRAFORM_DIR}/${ENVIRONMENT}"
    terraform init
fi

check_aws_credentials

# Get AWS account ID and region from AWS CLI
AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
AWS_REGION=${AWS_DEFAULT_REGION:-$(aws configure get region || echo "ap-northeast-1")}

log_info "AWS Account: ${AWS_ACCOUNT_ID}, Region: ${AWS_REGION}"

# Get ECR repository URLs from Terraform
log_info "Getting ECR repository URLs from Terraform..."
cd "${TERRAFORM_DIR}/${ENVIRONMENT}"

# Get ECR URLs
BACKEND_REPO=$(get_terraform_output "backend_repository_url")
FRONTEND_REPO=$(get_terraform_output "frontend_repository_url")
MPC_NODE_REPO=$(get_terraform_output "mpc_node_repository_url")

# Get ALB DNS name for frontend build args
ALB_DNS=$(get_terraform_output "alb_dns_name")

# Validate ECR URLs based on service
validate_repo() {
    local service=$1
    local repo=$2
    if [ -z "$repo" ]; then
        log_error "ECR repository URL for ${service} not found. Has Terraform been applied?"
    fi
}

# Login to ECR
log_info "Logging in to ECR..."
aws ecr get-login-password --region "${AWS_REGION}" | \
    docker login --username AWS --password-stdin "${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com" || \
    log_error "Failed to login to ECR"

# Function to build and push a service
build_and_push() {
    local service=$1
    local dockerfile=$2
    local repo=$3
    local tag=${4:-latest}
    shift 4
    local build_args=("$@")

    log_info "==================================="
    log_info "Processing: ${service}"
    log_info "==================================="

    # Validate repository URL
    validate_repo "${service}" "${repo}"

    cd "${PROJECT_ROOT}"

    # Build Docker image
    if [ "$SKIP_BUILD" = false ]; then
        log_info "Building ${service} Docker image..."

        # Prepare build args
        local build_cmd="docker build --platform linux/amd64 -f ${dockerfile} -t ${repo}:${tag}"
        for arg in "${build_args[@]}"; do
            build_cmd="${build_cmd} --build-arg ${arg}"
        done
        build_cmd="${build_cmd} ."

        if eval "${build_cmd}"; then
            log_info "Successfully built ${service} image"
        else
            log_error "Failed to build ${service} image"
        fi
    else
        log_info "Skipping build for ${service}"
    fi

    # Push to ECR
    log_info "Pushing ${service} image to ECR..."
    if docker push "${repo}:${tag}"; then
        log_info "Successfully pushed ${service} image to ${repo}:${tag}"
    else
        log_error "Failed to push ${service} image"
    fi

    echo ""
}

# Deploy services based on selection
case $SERVICE in
    backend)
        build_and_push "backend" "packages/server/Dockerfile" "${BACKEND_REPO}" "latest"
        ;;
    frontend)
        build_and_push "frontend" "packages/nextjs/Dockerfile" "${FRONTEND_REPO}" "latest" \
            "NEXT_PUBLIC_API_URL=http://${ALB_DNS}/api" \
            "NEXT_PUBLIC_WS_URL=ws://${ALB_DNS}/api"
        ;;
    mpc-node)
        build_and_push "mpc-node" "packages/zk-mpc-node/Dockerfile" "${MPC_NODE_REPO}" "latest"
        ;;
    all)
        build_and_push "backend" "packages/server/Dockerfile" "${BACKEND_REPO}" "latest"
        build_and_push "frontend" "packages/nextjs/Dockerfile" "${FRONTEND_REPO}" "latest" \
            "NEXT_PUBLIC_API_URL=http://${ALB_DNS}/api" \
            "NEXT_PUBLIC_WS_URL=ws://${ALB_DNS}/api"
        build_and_push "mpc-node" "packages/zk-mpc-node/Dockerfile" "${MPC_NODE_REPO}" "latest"
        ;;
    *)
        log_error "Unknown service: ${SERVICE}. Valid options: backend, frontend, mpc-node, all"
        ;;
esac

cat << EOF

${GREEN}===================================
Deployment completed successfully!${NC}
===================================

Next steps:
1. Update ECS services to use new images:
   ./scripts/update-ecs-service.sh ${ENVIRONMENT} ${SERVICE}

2. Monitor deployment:
   aws ecs describe-services --cluster zk-werewolf-${ENVIRONMENT}-cluster --services zk-werewolf-${ENVIRONMENT}-backend

3. View logs:
   aws logs tail /ecs/zk-werewolf-${ENVIRONMENT}-cluster --follow
EOF
