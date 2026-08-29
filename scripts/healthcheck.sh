#!/bin/bash
# healthcheck.sh — 健康检查

set -e

ENDPOINT="${1:-http://localhost:8080/health}"
TIMEOUT="${2:-10}"

echo "=== Health check: $ENDPOINT ==="

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time "$TIMEOUT" "$ENDPOINT" || echo "000")

if [ "$HTTP_CODE" = "200" ]; then
  echo "OK (HTTP $HTTP_CODE)"
  exit 0
else
  echo "FAIL (HTTP $HTTP_CODE)"
  exit 1
fi
