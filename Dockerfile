# 构建阶段：稳定版 Rust + GitHub 上游 topcoat（主分支，随上游更新）
FROM rustlang/rust:stable AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake perl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

# 本地开发用 path 依赖（../tmp/topcoat）；CI 构建上下文无该目录，改用 GitHub 上游
RUN sed -i 's|path = "../tmp/topcoat/crates/topcoat"|git = "https://github.com/tokio-rs/topcoat"|g' Cargo.toml

# 测试（纯函数测试无 DB 依赖，smoke 类自动忽略）；
# 用 --release：与后续 release 构建共享依赖编译，省一整轮 debug 编译
RUN cargo test --release

# 构建 + 资产打包：assets/ 由 topcoat asset bundle 生成在 exe 旁，cargo build 本身不产出
RUN cargo build --release \
    && cargo install --git https://github.com/tokio-rs/topcoat topcoat-cli \
    && topcoat asset bundle --release

# 运行阶段：精简镜像 + 运行时依赖（视频上传运行期 HLS 转码需 ffmpeg）
FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/mousuo /app/
COPY --from=builder /build/target/release/assets /app/assets

WORKDIR /app
ENV HOST=0.0.0.0
ENV PORT=7800
CMD ["./mousuo"]
