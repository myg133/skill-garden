//! 下载 handlers (Skill/CLI)

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse};

use crate::api::error::ApiError;
use super::helpers::ApiState;

pub async fn download_skill_handler(
    State(state): State<ApiState>,
    Path((name, version)): Path<(String, String)>,
    Query(query): Query<DownloadSkillQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. 闃叉璺緞閬嶅巻
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(ApiError::BadRequest("Invalid skill name".to_string()));
    }

    // 2. 从数据库验证并消费下载凭证
    let token_record = state
        .download_token_repo
        .validate_and_consume(&query.token, &name, &version)
        .await
        .map_err(|e| {
            tracing::error!("Download token DB lookup failed: {}", e);
            ApiError::InternalError("Download verification failed".to_string())
        })?
        .ok_or_else(|| {
            ApiError::Unauthorized("Invalid, expired, or already used download token".to_string())
        })?;

    tracing::info!(
        "Skill download: skill={}/v{}, identity={}, api_key={}",
        name,
        version,
        token_record.identity_id,
        token_record.api_key_id
    );

    let filename = format!("{}-{}.tar.gz", name, version);

    // 3. 优先使用预生成的 release tarball（审核通过后 git archive 生成）
    let release_tarball_path = state
        .skill_git
        .releases_dir()
        .join(&name)
        .join(format!("v{}.tar.gz", version));

    if release_tarball_path.exists() {
        let tarball = tokio::fs::read(&release_tarball_path).await.map_err(|e| {
            tracing::error!("Failed to read release tarball: {}", e);
            ApiError::InternalError("Failed to read release tarball".to_string())
        })?;

        tracing::info!(
            "Serving pre-built release tarball: {}",
            release_tarball_path.display()
        );

        let response = axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/gzip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            )
            .header("Content-Length", tarball.len().to_string())
            .body(axum::body::Body::from(tarball))
            .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

        return Ok(response);
    }

    // 4. 无预生成 tarball — 实时从 git archive 生成并缓存到 releases
    match state.skill_git.generate_release_tarball(&name, &version) {
        Ok(path) => {
            tracing::info!("Generated release tarball on demand: {}", path.display());
        }
        Err(e) => {
            tracing::error!(
                "Failed to generate release tarball for skill '{}' version {}: {}",
                name, version, e
            );
            return Err(ApiError::InternalError(format!(
                "Failed to generate release tarball: {}",
                e
            )));
        }
    }

    // 閲嶆柊璇诲彇鍒氱敓鎴愮殑 tarball
    if release_tarball_path.exists() {
        let tarball = tokio::fs::read(&release_tarball_path).await.map_err(|e| {
            tracing::error!("Failed to read release tarball: {}", e);
            ApiError::InternalError("Failed to read release tarball".to_string())
        })?;

        tracing::info!("Served freshly generated release tarball: {}", release_tarball_path.display());

        let response = axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/gzip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            )
            .header("Content-Length", tarball.len().to_string())
            .body(axum::body::Body::from(tarball))
            .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

        return Ok(response);
    }

    return Err(ApiError::NotFound(format!(
        "Release tarball not available for skill '{}' version {}",
        name, version
    )));
}

/// GET /api/v1/cli/download/:version/:target?token=...
/// 返回 CLI 的 tar.gz 包（含 binary + config.toml + install 脚本 + SKILL.md）
/// target 格式：{os}-{arch}，如 linux-x86_64、windows-x86_64
/// token 为 DB 中的不透明 UUID，由 cli.setup MCP 工具生成，10 分钟有效
pub async fn download_cli_handler(
    State(state): State<ApiState>,
    Path((version, target)): Path<(String, String)>,
    Query(query): Query<DownloadSkillQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. 闃叉璺緞閬嶅巻
    if version.contains("..")
        || version.contains('/')
        || version.contains('\\')
        || target.contains("..")
        || target.contains('/')
        || target.contains('\\')
    {
        return Err(ApiError::BadRequest(
            "Invalid version or target".to_string(),
        ));
    }

    // 2. 楠岃瘉 CLI 涓嬭浇鍑瘉
    let token_record = state
        .download_token_repo
        .validate_cli_token(&query.token)
        .await
        .map_err(|e| {
            tracing::error!("CLI download token DB lookup failed: {}", e);
            ApiError::InternalError("Download verification failed".to_string())
        })?
        .ok_or_else(|| {
            ApiError::Unauthorized(
                "Invalid, expired, or already used CLI download token".to_string(),
            )
        })?;

    tracing::info!(
        "CLI download: v{}/{}, identity={}, api_key={}",
        version,
        target,
        token_record.identity_id,
        token_record.api_key_id
    );

    // 3. 找到 CLI 二进制文件
    let is_windows = target.starts_with("windows");
    let binary_name = if is_windows {
        "skill-garden.exe"
    } else {
        "skill-garden"
    };

    let bin_path = std::path::PathBuf::from("cli-dist")
        .join(&version)
        .join(&target)
        .join(binary_name);

    if !bin_path.exists() {
        return Err(ApiError::NotFound(format!(
            "CLI binary v{}/{} not found on server. \
             Build it with: cargo build --release --no-default-features --features cli --bin skill-garden, \
             then place it at cli-dist/{}/{}/{}",
            version, target, version, target, binary_name
        )));
    }

    // 4. 读取预填的 config.toml（cli.setup 时写入 token）
    let config_data = token_record.config_data.unwrap_or_else(|| {
        let server_url = std::env::var("AION_HIVE_PUBLIC_URL").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}",
                std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
            )
        });
        format!(
            "server = \"{}\"\ntoken = \"sk_<YOUR_API_KEY>\"\n",
            server_url.trim_end_matches('/')
        )
    });

    let version_clone = version.clone();
    let target_clone = target.clone();

    // Compute display labels for SKILL.md template
    let server_url = std::env::var("AION_HIVE_PUBLIC_URL").unwrap_or_else(|_| {
        format!(
            "http://localhost:{}",
            std::env::var("AION_HIVE_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
        )
    });
    let os_label = if target.starts_with("linux") {
        "Linux"
    } else if target.starts_with("macos") {
        "macOS"
    } else {
        "Windows"
    };
    let verify_cmd = if is_windows {
        "skill-garden.exe whoami"
    } else {
        "skill-garden whoami"
    };

    // 5. 在 blocking 线程池中生成 tar.gz
    let tarball = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        let encoder = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(encoder);

        let prefix = "skill-garden-cli";

        // Helper: add a file from bytes
    fn add_bytes<W: std::io::Write>(
            tar: &mut tar::Builder<W>,
            path: &str,
            data: &[u8],
            mode: u32,
        ) -> Result<(), String> {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(path)
                .map_err(|e| format!("tar path error: {}", e))?;
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_cksum();
            tar.append_data(&mut header, path, std::io::Cursor::new(data))
                .map_err(|e| format!("tar append error for {}: {}", path, e))?;
            Ok(())
        }

        // 5a. 添加二进制文件
    let bin_bytes = std::fs::read(&bin_path)
            .map_err(|e| format!("Failed to read binary {}: {}", bin_path.display(), e))?;
        let bin_tar_path = format!("{}/{}", prefix, binary_name);
        add_bytes(&mut tar_builder, &bin_tar_path, &bin_bytes, 0o755)?;

        // 5b. 娣诲姞 config.toml
    let config_tar_path = format!("{}/config.toml", prefix);
        add_bytes(
            &mut tar_builder,
            &config_tar_path,
            config_data.as_bytes(),
            0o644,
        )?;

        // 5c. 娣诲姞 install.sh
    let install_sh =
            include_str!("../../../cli-dist/install.sh").replace("{version}", &version_clone);
        let install_sh_path = format!("{}/install.sh", prefix);
        add_bytes(
            &mut tar_builder,
            &install_sh_path,
            install_sh.as_bytes(),
            0o755,
        )?;

        // 5d. 娣诲姞 install.ps1
    let install_ps1 =
            include_str!("../../../cli-dist/install.ps1").replace("{version}", &version_clone);
        let install_ps1_path = format!("{}/install.ps1", prefix);
        add_bytes(
            &mut tar_builder,
            &install_ps1_path,
            install_ps1.as_bytes(),
            0o644,
        )?;

        // 5e. 添加 skill-garden/SKILL.md（作为独立 Skill 目录，Agent 可直接安装）
    let skill_md = include_str!("../../../cli-dist/SKILL.md")
            .replace("{server_url}", &server_url)
            .replace("{os}", os_label)
            .replace("{version}", &version_clone)
            .replace("{verify}", verify_cmd);
        add_bytes(
            &mut tar_builder,
            "skill-garden/SKILL.md",
            skill_md.as_bytes(),
            0o644,
        )?;

        // Finalize tar.gz
    let encoder = tar_builder
            .into_inner()
            .map_err(|e| format!("Failed to finalize tar: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Failed to compress: {}", e))?;

        Ok(buf)
    })
    .await
    .map_err(|e| ApiError::InternalError(format!("Tarball generation failed: {}", e)))?
    .map_err(|e| ApiError::InternalError(e))?;

    // 6. 返回 tar.gz 流
    let archive_name = format!("skill-garden-cli-{}-{}.tar.gz", target_clone, version);
    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/gzip")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", archive_name),
        )
        .header("Content-Length", tarball.len().to_string())
        .body(axum::body::Body::from(tarball))
        .map_err(|e| ApiError::InternalError(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// 下载参数
#[derive(serde::Deserialize)]
pub struct DownloadSkillQuery {
    pub token: String,
}





