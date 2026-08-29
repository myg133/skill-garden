#!/bin/bash
# deploy.sh — 通用部署脚本

set -e

ENV="${1:-staging}"
APP="${2:-aion-hive}"
IMAGE_TAG="${IMAGE_TAG:-develop-latest}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(dirname "$SCRIPT_DIR")"
ENV_FILE="$DEPLOY_DIR/environments/$ENV/.env"

if [ ! -f "$ENV_FILE" ]; then
  echo "Error: environment file not found: $ENV_FILE"
  exit 1
fi

echo "=== Deploying $APP to $ENV (tag: $IMAGE_TAG) ==="

# Load environment variables
set -a
source "$ENV_FILE"
set +a

# Update IMAGE_TAG from env or argument
IMAGE_TAG="${IMAGE_TAG}" kustomize edit set image "app=$APP:$IMAGE_TAG"

echo "Environment: $ENV"
echo "Image tag: $IMAGE_TAG"
echo "Deploying..."

# Placeholder for actual k8s/helm apply
echo "TODO: helm upgrade --install $APP ./apps/$APP -n $ENV -f $ENV_FILE"
