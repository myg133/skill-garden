#!/bin/bash
# ===========================================
# AionHive Helm 部署脚本
# ===========================================

set -e

CHART_PATH="$(cd "$(dirname "$0")/../apps/aion-hive" && pwd)"
NAMESPACE="${NAMESPACE:-aionhive}"
RELEASE_NAME="${RELEASE_NAME:-aionhive}"
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

# 预检查
precheck() {
    if ! command -v helm &>/dev/null; then
        error "Helm 未安装，请先安装: https://helm.sh/docs/intro/install/"
        exit 1
    fi
    
    if ! command -v kubectl &>/dev/null; then
        error "kubectl 未安装"
        exit 1
    fi
}

# 安装
install() {
    local env=$1
    local image_tag=$2
    
    precheck
    
    info "使用 Helm 安装 AionHive"
    info "环境: $env"
    info "镜像版本: $image_tag"
    
    # 构建命令
    local cmd="helm upgrade --install $RELEASE_NAME $CHART_PATH \
        --namespace $NAMESPACE \
        --create-namespace \
        --set image.tag=$image_tag"
    
    # 添加环境值文件
    if [ -f "$CHART_PATH/environments/values.$env.yaml" ]; then
        cmd="$cmd --values $CHART_PATH/environments/values.$env.yaml"
    fi
    
    # 交互式设置 Secret
    read -p "请输入 DATABASE_URL: " db_url
    read -p "请输入 JWT_SECRET (至少 32 字符): " jwt_secret
    read -p "请输入 API_KEY_SALT: " api_key_salt
    
    cmd="$cmd \
        --set secrets.databaseUrl='$db_url' \
        --set secrets.jwtSecret='$jwt_secret' \
        --set secrets.apiKeySalt='$api_key_salt'"
    
    # 执行
    info "执行: helm upgrade --install ..."
    eval $cmd
    
    info "等待 Pod 就绪..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s
    
    info "部署完成!"
    show_status
}

# 更新
upgrade() {
    local env=$1
    local image_tag=$2
    
    precheck
    
    info "使用 Helm 更新 AionHive"
    
    local cmd="helm upgrade $RELEASE_NAME $CHART_PATH \
        --namespace $NAMESPACE \
        --set image.tag=$image_tag"
    
    if [ -f "$CHART_PATH/environments/values.$env.yaml" ]; then
        cmd="$cmd --values $CHART_PATH/environments/values.$env.yaml"
    fi
    
    eval $cmd
    
    info "等待 Pod 就绪..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s
    
    info "更新完成!"
    show_status
}

# 卸载
uninstall() {
    precheck
    
    warn "即将卸载 AionHive (namespace: $NAMESPACE)"
    read -p "确认? (y/N): " confirm
    
    if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
        helm uninstall "$RELEASE_NAME" --namespace "$NAMESPACE"
        info "卸载完成"
    else
        info "取消卸载"
    fi
}

# 查看状态
show_status() {
    echo -e "\n=== Pods ==="
    kubectl get pods -n "$NAMESPACE" -l "app.kubernetes.io/name=aionhive"
    
    echo -e "\n=== Service ==="
    kubectl get svc -n "$NAMESPACE"
    
    echo -e "\n=== Ingress ==="
    kubectl get ingress -n "$NAMESPACE"
    
    echo -e "\n=== HPA ==="
    kubectl get hpa -n "$NAMESPACE" 2>/dev/null || echo "HPA 未启用"
    
    echo -e "\n=== PVC ==="
    kubectl get pvc -n "$NAMESPACE"
}

# 回滚
rollback() {
    precheck
    
    local revision=${1:-1}
    info "回滚 Helm release 到版本 $revision"
    
    helm rollback "$RELEASE_NAME" "$revision" --namespace "$NAMESPACE"
    kubectl rollout status deployment/aionhive -n "$NAMESPACE"
    
    info "回滚完成!"
}

# 显示帮助
help() {
    echo "AionHive Helm 部署工具"
    echo ""
    echo "用法: $0 <command> [options]"
    echo ""
    echo "命令:"
    echo "  install <env> <tag>  安装 (env: staging|production)"
    echo "  upgrade <env> <tag>  更新"
    echo "  uninstall             卸载"
    echo "  status               查看状态"
    echo "  rollback [rev]       回滚 (默认到上一个版本)"
    echo "  help                 显示帮助"
    echo ""
    echo "环境变量:"
    echo "  NAMESPACE            K8s namespace (默认: aionhive)"
    echo "  RELEASE_NAME         Helm release name (默认: aionhive)"
    echo "  KUBECTL_CONTEXT      kubectl context"
    echo ""
    echo "示例:"
    echo "  NAMESPACE=aionhive-staging $0 install staging staging-latest"
    echo "  $0 upgrade production v0.3.0"
    echo "  $0 status"
}

# 主入口
case "${1:-help}" in
    install)
        install "${2:-staging}" "${3:-latest}"
        ;;
    upgrade)
        upgrade "${2:-staging}" "${3:-latest}"
        ;;
    uninstall)
        uninstall
        ;;
    status)
        show_status
        ;;
    rollback)
        rollback "${2:-1}"
        ;;
    help|*)
        help
        ;;
esac
