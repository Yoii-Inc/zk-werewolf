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
  service        Service to update (backend, frontend, mpc-node, all) [default: all]

Options:
  --wait         Wait for deployment to complete
  --help         Show this help message

Examples:
  $0 dev                    # Update all services in dev
  $0 dev backend            # Update only backend in dev
  $0 dev backend --wait     # Update backend and wait for completion
EOF
    exit 1
}

# Parse arguments
ENVIRONMENT=""
SERVICE="all"
WAIT=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --wait)
            WAIT=true
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
check_aws_credentials

# Get AWS region from AWS CLI or environment
AWS_REGION=${AWS_DEFAULT_REGION:-$(aws configure get region || echo "ap-northeast-1")}

# Get cluster and service names from Terraform
log_info "Getting ECS configuration from Terraform..."
cd "${TERRAFORM_DIR}/${ENVIRONMENT}"

CLUSTER_NAME=$(get_terraform_output "ecs_cluster_name")
APPLICATION_URL=$(get_terraform_output "application_url")
API_URL=$(get_terraform_output "api_url")
WS_URL=$(get_terraform_output "ws_url")

if [ -z "$CLUSTER_NAME" ]; then
    log_error "Failed to get ECS cluster name from Terraform outputs"
fi

log_info "Cluster: ${CLUSTER_NAME}, Region: ${AWS_REGION}"

# Function to update a service
update_service() {
    local service=$1
    local defer_wait=${2:-false}
    local service_name="zk-werewolf-${ENVIRONMENT}-${service}"

    log_info "==================================="
    log_info "Updating: ${service_name}"
    log_info "==================================="

    # Check if service exists
    if ! aws ecs describe-services \
        --cluster "${CLUSTER_NAME}" \
        --services "${service_name}" \
        --region "${AWS_REGION}" \
        --query 'services[0].serviceName' \
        --output text 2>/dev/null | grep -q "${service_name}"; then
        log_warn "Service ${service_name} not found in cluster ${CLUSTER_NAME}"
        return 0
    fi

    # Force new deployment
    log_info "Forcing new deployment for ${service_name}..."
    if aws ecs update-service \
        --cluster "${CLUSTER_NAME}" \
        --service "${service_name}" \
        --force-new-deployment \
        --region "${AWS_REGION}" \
        --query 'service.deployments[0].status' \
        --output text > /dev/null; then
        log_info "Successfully triggered deployment for ${service_name}"
    else
        log_error "Failed to update ${service_name}"
    fi

    # Wait for deployment if requested (only when defer_wait=false)
    if [ "$WAIT" = true ] && [ "$defer_wait" != "true" ]; then
        log_info "Waiting for ${service_name} deployment to complete..."
        if aws ecs wait services-stable \
            --cluster "${CLUSTER_NAME}" \
            --services "${service_name}" \
            --region "${AWS_REGION}"; then
            log_info "${service_name} deployment completed successfully"
        else
            log_error "${service_name} deployment failed or timed out"
        fi
    else
        log_info "Deployment triggered. Use --wait to wait for completion."
    fi

    # Show current deployment status
    log_info "Current deployment status for ${service_name}:"
    aws ecs describe-services \
        --cluster "${CLUSTER_NAME}" \
        --services "${service_name}" \
        --region "${AWS_REGION}" \
        --query 'services[0].deployments[*].[id,status,desiredCount,runningCount,createdAt]' \
        --output table || log_warn "Failed to get deployment status"

    echo ""
}

# Function to update all MPC node services together in parallel
update_mpc_nodes_together() {
    local nodes=("mpc-node-0" "mpc-node-1" "mpc-node-2")
    local pids=()
    local full_service_names=()

    log_info "Triggering MPC nodes deployments in parallel..."

    for n in "${nodes[@]}"; do
        full_service_names+=("zk-werewolf-${ENVIRONMENT}-${n}")
        # defer wait so pass defer_wait=true
        ( update_service "$n" true ) &
        pids+=("$!")
    done

    # Collect results of parallel triggers
    local fail=0
    for pid in "${pids[@]}"; do
        if ! wait "$pid"; then
            fail=1
        fi
    done
    if [ "$fail" -ne 0 ]; then
        log_error "One or more MPC node deployments failed to trigger"
    fi

    # Service existence check: collect only existing services before running wait
    local existing_services=()
    for svc in "${full_service_names[@]}"; do
        if aws ecs describe-services \
            --cluster "${CLUSTER_NAME}" \
            --services "${svc}" \
            --region "${AWS_REGION}" \
            --query 'services[0].serviceName' \
            --output text 2>/dev/null | grep -q "${svc}"; then
            existing_services+=("${svc}")
        else
            log_warn "Service ${svc} not found in cluster ${CLUSTER_NAME}; skipping wait for this service."
        fi
    done

    if [ "${#existing_services[@]}" -eq 0 ]; then
        log_warn "No MPC node services found in cluster ${CLUSTER_NAME}; nothing to wait for."
        return 0
    fi

    # Finally wait together (wait only on existing services)
    if [ "$WAIT" = true ]; then
        log_info "Waiting for MPC nodes to become stable together: ${existing_services[*]}"
        if ! aws ecs wait services-stable \
            --cluster "${CLUSTER_NAME}" \
            --services "${existing_services[@]}" \
            --region "${AWS_REGION}"; then
            log_error "MPC nodes failed or timed out while waiting for stability"
        fi
        log_info "MPC nodes are stable."
    fi
}

# Update services based on selection
case $SERVICE in
    backend)
        update_service "backend"
        ;;
    frontend)
        update_service "frontend"
        ;;
    mpc-node|mpc-nodes)
        update_mpc_nodes_together
        ;;
    all)
        update_service "backend"
        update_service "frontend"
        update_mpc_nodes_together
        ;;
    *)
        log_error "Unknown service: ${SERVICE}. Valid options: backend, frontend, mpc-node, all"
        ;;
esac

log_info << EOF

===================================
Service update completed!
===================================

Monitor your deployment:
  aws ecs describe-services --cluster ${CLUSTER_NAME} --services zk-werewolf-${ENVIRONMENT}-${SERVICE} --region ${AWS_REGION}

View logs:
  aws logs tail /ecs/${CLUSTER_NAME} --follow --region ${AWS_REGION}
EOF

if [ -n "$APPLICATION_URL" ]; then
    log_info "Application URL: ${APPLICATION_URL}"
fi

if [ -n "$API_URL" ]; then
    log_info "API URL: ${API_URL}"
fi

if [ -n "$WS_URL" ]; then
    log_info "WebSocket URL: ${WS_URL}"
fi
