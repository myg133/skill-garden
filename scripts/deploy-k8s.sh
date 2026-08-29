#!/bin/bash
# ===========================================
# AionHive K8s 快速部署脚本
# 支持首次完整部署和增量更新
# ===========================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
K8S_DIR="$(cd "$SCRIPT_DIR/../k8s" && pwd)"
NAMESPACE="${NAMESPACE:-aionhive}"
CONTEXT="${KUBECTL_CONTEXT:-}"
NON_INTERACTIVE="${NON_INTERACTIVE:-false}"
DEPLOY_ALL="${DEPLOY_ALL:-false}"  # 首次部署时是否 apply 所有资源

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 参数解析
while [[ $# -gt 0 ]]; do
    case "$1" in
        --non-interactive)
            NON_INTERACTIVE=true
            shift
            ;;
        --deploy-all)
            DEPLOY_ALL=true
            shift
            ;;
        --namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        --context)
            CONTEXT="$2"
            shift 2
            ;;
        *)
            break
            ;;
    esac
done

COMMAND="${1:-help}"
ENV="${2:-staging}"
IMAGE_TAG="${3:-latest}"

# 使用指定 context
if [ -n "$CONTEXT" ]; then
    info "使用 context: $CONTEXT"
    kubectl config use-context "$CONTEXT" 2>/dev/null || true
fi

# 预检查
precheck() {
    if ! command -v kubectl &>/dev/null; then
        error "kubectl 未安装"
        exit 1
    fi
}

# 检查 namespace 是否存在
namespace_exists() {
    kubectl get namespace "$NAMESPACE" &>/dev/null
}

# 检查 secret 是否存在
secret_exists() {
    kubectl get secret aionhive-secret -n "$NAMESPACE" &>/dev/null
}

# 创建 namespace (如果不存在)
ensure_namespace() {
    if ! namespace_exists; then
        info "创建 namespace: $NAMESPACE"
        kubectl apply -f "$K8S_DIR/namespace.yaml" --namespace="$NAMESPACE" 2>/dev/null || \
        kubectl create namespace "$NAMESPACE"
    else
        info "Namespace $NAMESPACE 已存在"
    fi
}

# 确保 Secret 存在
ensure_secret() {
    if secret_exists; then
        info "Secret aionhive-secret 已存在，跳过创建"
        return 0
    fi

    warn "Secret 不存在，正在创建..."

    # 尝试从环境变量获取，或交互式输入
    local db_url="${DATABASE_URL:-}"
    local jwt_secret="${JWT_SECRET:-}"
    local api_key_salt="${API_KEY_SALT:-}"

    if [ "$NON_INTERACTIVE" = "true" ]; then
        if [ -z "$db_url" ] || [ -z "$jwt_secret" ] || [ -z "$api_key_salt" ]; then
            error "非交互模式下，必须设置环境变量: DATABASE_URL, JWT_SECRET, API_KEY_SALT"
            exit 1
        fi
    else
        if [ -z "$db_url" ]; then
            read -r -p "请输入 DATABASE_URL: " db_url
        fi
        if [ -z "$jwt_secret" ]; then
            read -r -p "请输入 JWT_SECRET (>=32 字符): " jwt_secret
        fi
        if [ -z "$api_key_salt" ]; then
            read -r -p "请输入 API_KEY_SALT: " api_key_salt
        fi
    fi

    kubectl create secret generic aionhive-secret -n "$NAMESPACE" \
        --from-literal="database-url=$db_url" \
        --from-literal="jwt-secret=$jwt_secret" \
        --from-literal="api-key-salt=$api_key_salt" \
        --dry-run=client -o yaml | kubectl apply -f -

    info "Secret 创建完成"
}

# 首次完整部署
deploy_all() {
    local tag=$1

    precheck
    ensure_namespace

    info "=== 首次完整部署 ==="
    info "应用所有 K8s 资源..."

    # 按顺序 apply 所有 manifest
    kubectl apply -f "$K8S_DIR/namespace.yaml"
    kubectl apply -f "$K8S_DIR/serviceaccount.yaml"
    kubectl apply -f "$K8S_DIR/configmap.yaml" 2>/dev/null || true
    ensure_secret

    # 应用 PVC (可选，storage provisioner 可能需要额外配置)
    if [ -f "$K8S_DIR/pvc.yaml" ]; then
        kubectl apply -f "$K8S_DIR/pvc.yaml" 2>/dev/null || \
            warn "PVC 创建失败 (可能需要先配置 StorageClass)"
    fi

    # 应用 Deployment
    kubectl apply -f "$K8S_DIR/deployment.yaml"

    # 更新镜像 tag
    info "更新镜像版本: $tag"
    kubectl set image deployment/aionhive aionhive=aionhive/aion-hive:"$tag" -n "$NAMESPACE"

    # 应用其他资源 (按依赖顺序)
    kubectl apply -f "$K8S_DIR/service.yaml"
    kubectl apply -f "$K8S_DIR/pdb.yaml" 2>/dev/null || true
    kubectl apply -f "$K8S_DIR/hpa.yaml" 2>/dev/null || true
    kubectl apply -f "$K8S_DIR/ingress.yaml" 2>/dev/null || true

    info "所有资源已应用"
}

# 增量更新部署
deploy_update() {
    local tag=$1

    precheck

    if ! namespace_exists; then
        warn "Namespace $NAMESPACE 不存在，执行首次完整部署 (添加 --deploy-all 跳过此提示)"
        deploy_all "$tag"
        return
    fi

    if ! secret_exists; then
        warn "Secret 不存在，创建..."
        ensure_secret
    fi

    info "=== 增量更新部署 ==="
    info "镜像版本: $tag"

    # 更新镜像
    kubectl set image deployment/aionhive aionhive=aionhive/aion-hive:"$tag" -n "$NAMESPACE"

    # 可选: 更新 ConfigMap
    if [ -f "$K8S_DIR/configmap.yaml" ]; then
        kubectl apply -f "$K8S_DIR/configmap.yaml"
    fi

    info "等待部署完成..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s || {
        error "部署超时，查看 Pod 状态:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
        kubectl describe pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive | tail -30
        exit 1
    }

    info "更新完成!"
}

# 部署入口
deploy() {
    local env=$1
    local tag=$2

    info "部署环境: $env, 镜像版本: $tag"

    if [ "$DEPLOY_ALL" = "true" ] || [ "$1" = "--deploy-all" ]; then
        deploy_all "$tag"
    else
        deploy_update "$tag"
    fi

    show_status
}

# 查看状态
status() {
    precheck

    echo "=== Namespace: $NAMESPACE ==="
    kubectl get namespace "$NAMESPACE" &>/dev/null || { echo "Namespace 不存在"; return 1; }

    echo -e "\n=== Pods ==="
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive

    echo -e "\n=== Service ==="
    kubectl get svc -n "$NAMESPACE"

    echo -e "\n=== Ingress ==="
    kubectl get ingress -n "$NAMESPACE" 2>/dev/null || echo "无 Ingress"

    echo -e "\n=== HPA ==="
    kubectl get hpa -n "$NAMESPACE" 2>/dev/null || echo "HPA 未启用"

    echo -e "\n=== PVC ==="
    kubectl get pvc -n "$NAMESPACE" 2>/dev/null || echo "无 PVC"

    echo -e "\n=== Events (最近) ==="
    kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' 2>/dev/null | tail -10
}

# 回滚
rollback() {
    precheck

    local revision=${1:-1}
    info "回滚到版本 $revision"
    kubectl rollout undo deployment/aionhive -n "$NAMESPACE" --to-revision="$revision"
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s
    info "回滚完成"
    show_status
}

# 日志
logs() {
    precheck

    local follow=""
    local lines="100"
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -f|--follow) follow="-f"; shift ;;
            -n|--lines) lines="$2"; shift 2 ;;
            *) break ;;
        esac
    done

    kubectl logs -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive --tail="$lines" $follow
}

# 清理
cleanup() {
    precheck

    warn "即将删除 $NAMESPACE namespace 下的所有 AionHive 资源"
    if [ "$NON_INTERACTIVE" = "true" ]; then
        warn "非交互模式，跳过确认"
    else
        read -r -p "输入 'yes' 确认: " confirm
        if [ "$confirm" != "yes" ]; then
            info "取消清理"
            return
        fi
    fi

    kubectl delete namespace "$NAMESPACE"
    info "清理完成"
}

# 显示帮助
help() {
    echo "AionHive K8s 部署工具"
    echo ""
    echo "用法: $0 <command> [options] [env] [tag]"
    echo ""
    echo "命令:"
    echo "  deploy <env> <tag>   部署"
    echo "  status                查看部署状态"
    echo "  rollback [rev]        回滚 (默认到上一个版本)"
    echo "  logs [-f] [-n N]     查看日志 (-f 实时, -n 行数)"
    echo "  cleanup               删除所有资源"
    echo "  help                  显示帮助"
    echo ""
    echo "选项:"
    echo "  --deploy-all          首次部署时 apply 所有资源 (用于新环境)"
    echo "  --non-interactive     非交互模式 (需要设置环境变量)"
    echo "  --namespace <ns>      K8s namespace (默认: aionhive)"
    echo "  --context <ctx>      kubectl context"
    echo ""
    echo "环境变量 (非交互模式必须设置):"
    echo "  DATABASE_URL          数据库连接字符串"
    echo "  JWT_SECRET            JWT 密钥 (>= 32 字符)"
    echo "  API_KEY_SALT         API Key 盐值"
    echo ""
    echo "示例:"
    echo "  # 首次部署 (新环境)"
    echo "  $0 --deploy-all staging v0.3.0"
    echo ""
    echo "  # 增量更新 (已有环境)"
    echo "  $0 deploy staging v0.3.0"
    echo ""
    echo "  # 非交互模式 (CI/CD)"
    echo "  DATABASE_URL='...' JWT_SECRET='...' API_KEY_SALT='...' \\"
    echo "    $0 --non-interactive deploy production v0.3.0"
    echo ""
    echo "  # 查看日志"
    echo "  $0 logs -f -n 200"
}

# 显示状态 (简版)
show_status() {
    echo -e "\n=== Pods ==="
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive 2>/dev/null || true
}

# 主入口
case "$COMMAND" in
    deploy)
        deploy "$ENV" "$IMAGE_TAG"
        ;;
    status)
        status
        ;;
    rollback)
        rollback "${2:-1}"
        ;;
    logs)
        shift
        logs "$@"
        ;;
    cleanup)
        cleanup
        ;;
    help|*)
        help
        ;;
esac
