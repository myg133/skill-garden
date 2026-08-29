# ===========================================
# AionHive Dockerfile
# 多阶段构建优化镜像大小
# ===========================================

# -----------------------------------------
# Stage 1: Builder
# -----------------------------------------
FROM rust:1.75-slim as builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# 复制依赖文件
COPY Cargo.toml Cargo.lock* ./

# 创建假的 main.rs 以缓存依赖
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/lib.rs

# 缓存依赖
RUN cargo build --release && \
    rm -rf src

# 复制源码
COPY src/ src/

# 重新构建 (包含实际代码)
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --bin server

# -----------------------------------------
# Stage 2: CLI Builder
# -----------------------------------------
FROM rust:1.75-slim as cli-builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock* ./

RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs

RUN cargo build --release --features cli

# -----------------------------------------
# Stage 3: Runtime
# -----------------------------------------
FROM debian:bookworm-slim as runtime

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN groupadd --gid 1000 aionhive && \
    useradd --uid 1000 --gid aionhive --shell /bin/bash --create-home aionhive

WORKDIR /app

# 复制二进制文件
COPY --from=builder /build/target/release/server /app/server
COPY --from=cli-builder /build/target/release/skill-garden /app/skill-garden

# 复制 CLI 资源文件
COPY --from=builder /build/target/release/cli-dist /app/cli-dist

# 创建数据目录
RUN mkdir -p /data/registry /data/evaluations /data/search_index /skills && \
    chown -R aionhive:aionhive /app /data /skills

# 复制健康检查脚本
COPY --from=builder /build/target/release/server /app/healthcheck

USER aionhive

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 默认端口
EXPOSE 8080

# 启动命令
CMD ["/app/server"]
