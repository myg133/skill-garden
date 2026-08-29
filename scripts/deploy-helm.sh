#!/bin/bash
# ===========================================
# AionHive Helm 部署脚本
# ===========================================

set -e

CHART_PATH="$(cd "$(dirname "$0")/../apps/aion-hive" && pwd)"
NAMESPACE="${NAMESPACE:-aionhive}"
RELEASE_NAME="${RELEASE_NAME:-aionhive}"
CONTEXT="${KUBECTL_CONTEXT:-}"
NON_INTERACTIVE="${NON_INTERACTIVE:-false}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 解析参数
while [[ $# -gt 0 ]]; do
    case "$1" in
        --non-interactive)
            NON_INTERACTIVE=true
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
    if ! command -v helm &>/dev/null; then
        error "Helm 未安装，请先安装: https://helm.sh/docs/intro/install/"
        exit 1
    fi

    if ! command -v kubectl &>/dev/null; then
        error "kubectl 未安装"
        exit 1
    fi

    # 检查 namespace 是否存在
    if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
        info "Namespace $NAMESPACE 不存在，将自动创建"
    fi
}

# 获取 Secret 值 (环境变量优先，交互式 fallback)
get_secret() {
    local name="$1"
    local env_var="$2"
    local hint="$3"
    local default="$4"

    local value="${!env_var:-}"

    if [ -n "$value" ]; then
        return 0
    fi

    if [ "$NON_INTERACTIVE" = "true" ]; then
        # 非交互模式: 使用默认值或报错
        if [ -n "$default" ]; then
            warn "环境变量 $env_var 未设置，使用默认值 (仅用于开发!)"
            echo "$default"
            return 0
        else
            error "环境变量 $env_var 未设置，请设置后再执行 (或移除 --non-interactive 交互输入)"
            exit 1
        fi
    fi

    # 交互模式
    local prompt="请输入 $name"
    if [ -n "$hint" ]; then
        prompt="$prompt ($hint)"
    fi
    prompt="$prompt: "

    local input
    read -r -p "$prompt" input
    if [ -z "$input" ] && [ -n "$default" ]; then
        input="$default"
    fi
    echo "$input"
}

# 构建 Helm set 参数
build_secrets() {
    local db_url
    local jwt_secret
    local api_key_salt

    db_url=$(get_secret "DATABASE_URL" "DATABASE_URL" "postgres://user:pass@host:5432/db" "")
    jwt_secret=$(get_secret "JWT_SECRET (至少 32 字符)" "JWT_SECRET" ">=32 chars" "")
    api_key_salt=$(get_secret "API_KEY_SALT" "API_KEY_SALT" "" "")

    # 验证 JWT_SECRET 长度
    if [ ${#jwt_secret} -lt 32 ]; then
        warn "JWT_SECRET 长度不足 32 字符，建议使用更长的密钥"
    fi

    echo "--set secrets.databaseUrl='$db_url' --set secrets.jwtSecret='$jwt_secret' --set secrets.apiKeySalt='$api_key_salt'"
}

# 安装
install() {
    local env=$1
    local image_tag=$2

    precheck

    info "使用 Helm 安装 AionHive"
    info "环境: $env"
    info "镜像版本: $image_tag"
    info "Namespace: $NAMESPACE"

    local secrets_set
    secrets_set=$(build_secrets)

    local cmd="helm upgrade --install $RELEASE_NAME $CHART_PATH \
        --namespace $NAMESPACE \
        --create-namespace \
        --set image.tag=$image_tag"

    if [ -f "$CHART_PATH/environments/values.$env.yaml" ]; then
        cmd="$cmd --values $CHART_PATH/environments/values.$env.yaml"
    fi

    cmd="$cmd $secrets_set"

    info "执行: helm upgrade --install ..."
    eval "$cmd"

    info "等待 Pod 就绪..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s || {
        error "部署超时，查看 Pod 状态:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
        kubectl describe pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive | tail -50
        exit 1
    }

    info "部署完成!"
    show_status
}

# 更新
upgrade() {
    local env=$1
    local image_tag=$2

    precheck

    info "使用 Helm 更新 AionHive"

    local secrets_set
    secrets_set=$(build_secrets)

    local cmd="helm upgrade $RELEASE_NAME $CHART_PATH \
        --namespace $NAMESPACE \
        --set image.tag=$image_tag"

    if [ -f "$CHART_PATH/environments/values.$env.yaml" ]; then
        cmd="$cmd --values $CHART_PATH/environments/values.$env.yaml"
    fi

    cmd="$cmd $secrets_set"

    eval "$cmd"

    info "等待 Pod 就绪..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s || {
        error "更新超时，查看 Pod 状态:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
        exit 1
    }

    info "更新完成!"
    show_status
}

# 卸载
uninstall() {
    precheck

    warn "即将卸载 AionHive (namespace: $NAMESPACE)"

    if [ "$NON_INTERACTIVE" = "true" ]; then
        warn "非交互模式，跳过确认，直接卸载"
        helm uninstall "$RELEASE_NAME" --namespace "$NAMESPACE"
        info "卸载完成"
    else
        read -r -p "确认卸载? (输入 'yes' 确认): " confirm
        if [ "$confirm" = "yes" ]; then
            helm uninstall "$RELEASE_NAME" --namespace "$NAMESPACE"
            info "卸载完成"
        else
            info "取消卸载"
        fi
    fi
}

# 查看状态
show_status() {
    echo -e "\n=== Pods ==="
    kubectl get pods -n "$NAMESPACE" -l "app.kubernetes.io/name=aionhive" 2>/dev/null || echo "No pods found"

    echo -e "\n=== Service ==="
    kubectl get svc -n "$NAMESPACE" 2>/dev/null || echo "No services found"

    echo -e "\n=== Ingress ==="
    kubectl get ingress -n "$NAMESPACE" 2>/dev/null || echo "No ingress found"

    echo -e "\n=== HPA ==="
    kubectl get hpa -n "$NAMESPACE" 2>/dev/null || echo "HPA 未启用"

    echo -e "\n=== PVC ==="
    kubectl get pvc -n "$NAMESPACE" 2>/dev/null || echo "No PVCs found"
}

# 回滚
rollback() {
    precheck

    local revision=${1:-1}
    info "回滚 Helm release 到版本 $revision"

    helm rollback "$RELEASE_NAME" "$revision" --namespace "$NAMESPACE"
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s

    info "回滚完成!"
    show_status
}

# 显示帮助
help() {
    echo "AionHive Helm 部署工具"
    echo ""
    echo "用法: $0 <command> [options] [env] [tag]"
    echo ""
    echo "命令:"
    echo "  install <env> <tag>  安装 (env: staging|production)"
    echo "  upgrade <env> <tag>  更新"
    echo "  uninstall            卸载"
    echo "  status               查看状态"
    echo "  rollback [rev]       回滚 (默认到上一个版本)"
    echo "  help                 显示帮助"
    echo ""
    echo "全局选项:"
    echo "  --non-interactive    非交互模式，使用环境变量输入 Secret"
    echo "  --namespace <ns>    K8s namespace (默认: aionhive)"
    echo "  --context <ctx>      kubectl context"
    echo ""
    echo "环境变量 (非交互模式必须设置):"
    echo "  DATABASE_URL         数据库连接字符串"
    echo "  JWT_SECRET           JWT 密钥 (建议 >= 32 字符)"
    echo "  API_KEY_SALT         API Key 盐值"
    echo ""
    echo "其他环境变量:"
    echo "  NAMESPACE            K8s namespace (默认: aionhive)"
    echo "  RELEASE_NAME         Helm release name (默认: aionhive)"
    echo "  KUBECTL_CONTEXT      kubectl context"
    echo ""
    echo "示例 — CI/CD 流水线 (非交互):"
    echo "  DATABASE_URL='postgres://...' JWT_SECRET='...' API_KEY_SALT='...' \\"
    echo "    NON_INTERACTIVE=true $0 install staging staging-latest"
    echo ""
    echo "  DATABASE_URL='postgres://...' JWT_SECRET='...' API_KEY_SALT='...' \\"
    echo "    $0 --non-interactive upgrade production v0.3.0"
    echo ""
    echo "示例 — 交互模式:"
    echo "  $0 install staging staging-latest"
    echo "  $0 upgrade production v0.3.0"
}

# 主入口
case "$COMMAND" in
    install)
        install "$ENV" "$IMAGE_TAG"
        ;;
    upgrade)
        upgrade "$ENV" "$IMAGE_TAG"
        ;;
    uninstall)
        uninstall
        ;;
    status)
        precheck
        show_status
        ;;
    rollback)
        precheck
        rollback "${2:-1}"
        ;;
    help|*)
        help
        ;;
esac
