#!/bin/bash
# ===========================================
# AionHive 回滚脚本
# 支持 Helm 和 K8s 两种部署方式
# ===========================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(dirname "$SCRIPT_DIR")"
NAMESPACE="${NAMESPACE:-aionhive}"
DEPLOY_METHOD="${DEPLOY_METHOD:-auto}"  # auto | helm | k8s
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
    kubectl config use-context "$CONTEXT" 2>/dev/null || true
fi

# 检测部署方式
detect_deploy_method() {
    if [ "$DEPLOY_METHOD" != "auto" ]; then
        echo "$DEPLOY_METHOD"
        return
    fi

    # 检查是否为 Helm release
    if helm list -n "$NAMESPACE" 2>/dev/null | grep -q "aionhive"; then
        echo "helm"
        return
    fi

    # 检查是否为 K8s Deployment (无 Helm)
    if kubectl get deployment aionhive -n "$NAMESPACE" &>/dev/null; then
        echo "k8s"
        return
    fi

    echo ""
}

# 预检查
precheck() {
    if ! command -v kubectl &>/dev/null; then
        error "kubectl 未安装"
        exit 1
    fi

    if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
        error "Namespace $NAMESPACE 不存在，请先部署"
        exit 1
    fi
}

# ===========================================
# Helm 回滚
# ===========================================
rollback_helm() {
    local target_rev=${1:-}
    local release_name="${RELEASE_NAME:-aionhive}"

    info "使用 Helm 回滚"

    # 列出可用版本
    echo -e "\n=== 可用 Helm revisions ==="
    helm history "$release_name" -n "$NAMESPACE" 2>/dev/null || {
        error "无法获取 Helm 历史"
        exit 1
    }

    # 确定回滚目标
    local revision=""
    if [ -n "$target_rev" ]; then
        revision="$target_rev"
    else
        # 回滚到上一个成功版本
        revision=$(helm history "$release_name" -n "$NAMESPACE" --output json 2>/dev/null \
            | grep -o '"revision":"[^"]*"' | head -2 | tail -1 | cut -d'"' -f4)
        if [ -z "$revision" ]; then
            revision=1
        fi
    fi

    info "回滚 $release_name 到 revision $revision"

    helm rollback "$release_name" "$revision" -n "$NAMESPACE"

    info "等待 Rollout 完成..."
    kubectl rollout status deployment/aionhive -n "$NAMESPACE" --timeout=300s || {
        error "Rollout 超时，查看 Pod 状态:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
        exit 1
    }

    info "Helm 回滚完成!"
    show_status
}

# ===========================================
# K8s 回滚
# ===========================================
rollback_k8s() {
    local target_rev=${1:-}

    info "使用 kubectl 回滚 (K8s Deployment)"

    local deployment_name="aionhive"

    if ! kubectl get deployment "$deployment_name" -n "$NAMESPACE" &>/dev/null; then
        deployment_name="${RELEASE_NAME:-aionhive}"
    fi

    if ! kubectl get deployment "$deployment_name" -n "$NAMESPACE" &>/dev/null; then
        error "Deployment $deployment_name 在 namespace $NAMESPACE 中不存在"
        exit 1
    fi

    # 列出可用版本
    echo -e "\n=== Deployment rollout history ==="
    kubectl rollout history deployment/"$deployment_name" -n "$NAMESPACE"

    # 确定回滚目标
    if [ -z "$target_rev" ]; then
        # 回滚到上一个版本
        info "回滚到上一个版本"
        kubectl rollout undo deployment/"$deployment_name" -n "$NAMESPACE"
    else
        info "回滚到版本 $target_rev"
        kubectl rollout undo deployment/"$deployment_name" -n "$NAMESPACE" --to-revision="$target_rev"
    fi

    info "等待 Rollout 完成..."
    kubectl rollout status deployment/"$deployment_name" -n "$NAMESPACE" --timeout=300s || {
        error "Rollout 超时，查看 Pod 状态:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive
        kubectl describe pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive | tail -30
        exit 1
    }

    info "K8s 回滚完成!"
    show_status
}

# 回滚 (主入口)
rollback_main() {
    local target_rev=${1:-}

    precheck

    local method
    method=$(detect_deploy_method)

    case "$method" in
        helm)
            rollback_helm "$target_rev"
            ;;
        k8s)
            rollback_k8s "$target_rev"
            ;;
        *)
            error "无法检测部署方式，请设置 DEPLOY_METHOD=helm 或 DEPLOY_METHOD=k8s"
            error "或者确保 namespace $NAMESPACE 中有 aionhive deployment/release"
            exit 1
            ;;
    esac
}

# 显示状态
show_status() {
    echo -e "\n=== Pods ==="
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=aionhive 2>/dev/null || \
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/instance=aionhive 2>/dev/null || \
        echo "No pods found"

    echo -e "\n=== Recent Events ==="
    kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' 2>/dev/null | tail -10 || true
}

# 显示帮助
help() {
    echo "AionHive 回滚工具"
    echo ""
    echo "用法: $0 [options] [revision]"
    echo ""
    echo "参数:"
    echo "  revision     回滚目标版本号 (不填则回滚到上一个版本)"
    echo ""
    echo "选项:"
    echo "  --method <helm|k8s|auto>  指定回滚方式 (默认: auto 检测)"
    echo "  --namespace <ns>          K8s namespace (默认: aionhive)"
    echo "  --context <ctx>           kubectl context"
    echo ""
    echo "环境变量:"
    echo "  DEPLOY_METHOD   回滚方式: helm | k8s | auto (默认: auto)"
    echo "  NAMESPACE       K8s namespace (默认: aionhive)"
    echo "  RELEASE_NAME    Helm release name (默认: aionhive)"
    echo "  KUBECTL_CONTEXT kubectl context"
    echo ""
    echo "示例:"
    echo "  $0                                          # 回滚到上一个版本 (auto)"
    echo "  $0 3                                        # 回滚到 revision 3"
    echo "  DEPLOY_METHOD=helm $0 2                    # 使用 Helm 回滚到版本 2"
    echo "  DEPLOY_METHOD=k8s $0                        # 使用 kubectl 回滚"
    echo "  NAMESPACE=aionhive-prod $0 1               # 回滚 production"
}

# 参数解析
while [[ $# -gt 0 ]]; do
    case "$1" in
        --method)
            DEPLOY_METHOD="$2"
            shift 2
            ;;
        --namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        --context)
            CONTEXT="$2"
            shift 2
            ;;
        --help|-h)
            help
            exit 0
            ;;
        -*)
            error "未知选项: $1"
            help
            exit 1
            ;;
        *)
            break
            ;;
    esac
done

REVISION="${1:-}"

if [ "$1" = "help" ]; then
    help
else
    rollback_main "$REVISION"
fi
