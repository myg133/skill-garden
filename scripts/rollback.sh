#!/bin/bash
# rollback.sh — 回滚到上一版本

set -e

ENV="${1:-staging}"
APP="${2:-aion-hive}"
PREV_TAG="${3:-}"

if [ -z "$PREV_TAG" ]; then
  echo "Usage: rollback.sh <env> <app> <prev_tag>"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(dirname "$SCRIPT_DIR")"
ENV_FILE="$DEPLOY_DIR/environments/$ENV/.env"

echo "=== Rolling back $APP in $ENV to $PREV_TAG ==="

# Placeholder for actual k8s/helm rollback
echo "TODO: helm rollback $APP -n $ENV"
