#!/bin/bash
# Common functions for deployment scripts

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
# Usage: log_info "message" or log_info << EOF
log_info() {
    if [ -t 0 ]; then
        # stdin is a terminal (argument provided)
        echo -e "${GREEN}[INFO]${NC} $1"
    else
        # stdin is not a terminal (heredoc or pipe)
        while IFS= read -r line; do
            echo -e "${GREEN}${line}${NC}"
        done
    fi
}

log_warn() {
    if [ -t 0 ]; then
        echo -e "${YELLOW}[WARN]${NC} $1"
    else
        while IFS= read -r line; do
            echo -e "${YELLOW}${line}${NC}"
        done
    fi
}

log_error() {
    if [ -t 0 ]; then
        echo -e "${RED}[ERROR]${NC} $1"
    else
        while IFS= read -r line; do
            echo -e "${RED}${line}${NC}"
        done
    fi
    exit 1
}

log_debug() {
    if [ -t 0 ]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    else
        while IFS= read -r line; do
            echo -e "${BLUE}${line}${NC}"
        done
    fi
}

# Check AWS credentials
check_aws_credentials() {
    log_info "Checking AWS credentials..."
    if ! aws sts get-caller-identity >/dev/null 2>&1; then
        log_error "No AWS credentials found. Please set AWS_PROFILE or configure AWS credentials."
    fi
}

# Validate environment directory exists
validate_environment() {
    local terraform_dir=$1
    local environment=$2

    if [ ! -d "${terraform_dir}/${environment}" ]; then
        log_error "Environment '${environment}' not found in ${terraform_dir}"
    fi
}

# Get Terraform output value
get_terraform_output() {
    local key=$1
    local default=${2:-""}
    terraform output -raw "${key}" 2>/dev/null || echo "${default}"
}
