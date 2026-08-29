#!/bin/bash
# ===========================================
# AionHive K8s 快速部署脚本
# ===========================================

set -e

NAMESPACE="aionhive"
CONTEXT="${KUBECTL_CONTEXT:-}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 使用指定 context
if [ -n "$CONTEXT" ]; then
    info "使用 context: $CONTEXT"
    kubectl config use-context "$CONTEXT"
fi

# 部署函数
deploy() {
    local env=$1
    local tag=$2
    
    info "部署环境: $env, 镜像版本: $tag"
    
    # 创建 namespace
    kubectl apply -f namespace.yaml
    
    # 创建 secret (如果不存在)
    if ! kubectl get secret aionhive-secret -n "$NAMESPACE" &>/dev/null; then
        warn "Secret 不存在，请手动创建:"
        echo "kubectl create secret generic aionhive-secret -n $NAMESPACE \\"
        echo "  --from-literal=database-url='postgres://aionhive:PASSWORD@HOST:5432/aionhive' \\"
        echo "  --from-literal=jwt-secret='YOUR_JWT_SECRET' \\"
        echo "  --from-literal=api-key-salt='YOUR_API_KEY_SALT'"
        exit 1
    fi
    
    # 应用配置
    kubectl set image deployment/aionhive aionhive=aionhive/aion-hive:$tag -n "$NAMESPACE"
    
    # 等待部署完成
    info "等待部署完成..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s
    
    # 显示状态
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
    
    info "部署完成!"
}

# 查看状态
status() {
    echo "=== Pods ==="
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
    
    echo -e "\n=== Service ==="
    kubectl get svc -n "$NAMESPACE"
    
    echo -e "\n=== Ingress ==="
    kubectl get ingress -n "$NAMESPACE"
    
    echo -e "\n=== HPA ==="
    kubectl get hpa -n "$NAMESPACE" 2>/dev/null || echo "HPA 未启用"
}

# 回滚
rollback() {
    local revision=${1:-1}
    info "回滚到版本 $revision"
    kubectl rollout undo deployment/aionhive -n "$NAMESPACE" --to-revision=$revision
    kubectl rollout status deployment/aionhive -n "$NAMESPACE"
    info "回滚完成"
}

# 日志
logs() {
    kubectl logs -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive --tail=100 -f
}

# 显示帮助
help() {
    echo "AionHive K8s 部署工具"
    echo ""
    echo "用法: $0 <command> [options]"
    echo ""
    echo "命令:"
    echo "  deploy <env> <tag>   部署 (env: staging|production)"
    echo "  status               查看部署状态"
    echo "  rollback [rev]       回滚 (默认到上一个版本)"
    echo "  logs                 查看日志"
    echo "  help                 显示帮助"
    echo ""
    echo "示例:"
    echo "  $0 deploy production v0.3.0"
    echo "  $0 rollback 2"
    echo "  $0 status"
}

# 主入口
case "${1:-help}" in
    deploy)
        deploy "${2:-staging}" "${3:-latest}"
        ;;
    status)
        status
        ;;
    rollback)
        rollback "${2:-1}"
        ;;
    logs)
        logs
        ;;
    help|*)
        help
        ;;
esac
