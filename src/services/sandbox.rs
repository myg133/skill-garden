//! Sandbox Service - Executes Org CLI tools in isolated Docker containers
//!
//! This service provides isolated execution environment for org tools and platform tools.
//! Tools are packaged as Docker images and executed via bollard SDK.
//!
//! Docker connection: TCP socket (default: tcp://localhost:2375)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StartContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use dashmap::DashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::models::error::AppError;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub docker_host: String,
    pub default_timeout: u64,
    pub max_container_lifetime: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            docker_host: std::env::var("DOCKER_HOST")
                .unwrap_or_else(|_| "http://localhost:2375".to_string()),
            default_timeout: 30,
            max_container_lifetime: 3600,
        }
    }
}

/// Tool execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_id: String,
    pub org_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_seconds: u64,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Sandbox status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStatus {
    Starting,
    Ready,
    Busy,
    Stopped,
    Error,
}

impl std::fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxStatus::Starting => write!(f, "starting"),
            SandboxStatus::Ready => write!(f, "ready"),
            SandboxStatus::Busy => write!(f, "busy"),
            SandboxStatus::Stopped => write!(f, "stopped"),
            SandboxStatus::Error => write!(f, "error"),
        }
    }
}

/// Sandbox instance info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub id: String,
    pub session_id: String,
    pub container_id: String,
    pub image: String,
    pub status: SandboxStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

/// Platform tool configuration
#[derive(Debug, Clone)]
pub struct PlatformTool {
    pub image: String,
    pub command: Vec<String>,
    pub timeout_seconds: u64,
}

impl PlatformTool {
    pub fn new(image: &str, command: Vec<&str>, timeout_seconds: u64) -> Self {
        Self {
            image: image.to_string(),
            command: command.into_iter().map(String::from).collect(),
            timeout_seconds,
        }
    }
}

/// SandboxService provides isolated execution environment for org tools.
/// Tools are packaged as Docker images and executed via bollard SDK.
#[derive(Debug, Clone)]
pub struct SandboxService {
    docker: Docker,
    containers: Arc<DashMap<String, SandboxInstance>>,
    platform_tools: Arc<HashMap<String, PlatformTool>>,
    default_timeout: u64,
    #[allow(dead_code)]
    max_container_lifetime: u64,
}

#[derive(Debug)]
struct SandboxInstance {
    info: SandboxInfo,
}

impl SandboxService {
    pub fn new() -> Self {
        Self::with_config(SandboxConfig::default())
    }

    pub fn with_config(config: SandboxConfig) -> Self {
        let mut platform_tools = HashMap::new();
        platform_tools.insert(
            "browse".to_string(),
            PlatformTool::new("aion-hive/tool-browse:latest", vec!["browse"], 30),
        );
        platform_tools.insert(
            "qa".to_string(),
            PlatformTool::new("aion-hive/tool-qa:latest", vec!["qa"], 60),
        );
        platform_tools.insert(
            "exec".to_string(),
            PlatformTool::new("aion-hive/tool-exec:latest", vec!["exec"], 120),
        );
        platform_tools.insert(
            "storage".to_string(),
            PlatformTool::new("aion-hive/tool-storage:latest", vec!["storage"], 30),
        );

        let docker = Docker::connect_with_http_defaults().expect("Failed to connect to Docker");

        Self {
            docker,
            containers: Arc::new(DashMap::new()),
            platform_tools: Arc::new(platform_tools),
            default_timeout: config.default_timeout,
            max_container_lifetime: config.max_container_lifetime,
        }
    }

    /// Execute an org tool in a sandboxed Docker container
    pub async fn execute_org_tool(
        &self,
        request: ToolExecutionRequest,
    ) -> Result<ToolExecutionResult, AppError> {
        let start = Instant::now();
        let timeout = if request.timeout_seconds > 0 {
            request.timeout_seconds
        } else {
            self.default_timeout
        };

        let sandbox = self
            .get_or_create_sandbox(&request.org_id, &request.tool_id)
            .await?;

        let result = self
            .execute_in_container(&sandbox, &request.parameters, timeout)
            .await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => Ok(ToolExecutionResult {
                success: true,
                output: Some(output),
                error: None,
                execution_time_ms,
            }),
            Err(e) => Ok(ToolExecutionResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                execution_time_ms,
            }),
        }
    }

    /// Execute a platform tool (browse, qa, exec, etc.)
    pub async fn execute_platform_tool(
        &self,
        tool_id: &str,
        parameters: HashMap<String, serde_json::Value>,
        timeout_seconds: Option<u64>,
    ) -> Result<ToolExecutionResult, AppError> {
        let tool = self.platform_tools.get(tool_id).ok_or_else(|| {
            AppError::ValidationError(format!("Unknown platform tool: {}", tool_id))
        })?;

        let timeout = timeout_seconds.unwrap_or(tool.timeout_seconds);
        let start = Instant::now();

        let sandbox = self
            .get_or_create_platform_sandbox(tool_id, &tool.image)
            .await?;

        let result = self
            .execute_in_container(&sandbox, &parameters, timeout)
            .await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => Ok(ToolExecutionResult {
                success: true,
                output: Some(output),
                error: None,
                execution_time_ms,
            }),
            Err(e) => Ok(ToolExecutionResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                execution_time_ms,
            }),
        }
    }

    /// Get or create a sandbox for an org tool
    async fn get_or_create_sandbox(&self, org_id: &str, tool_id: &str) -> Result<String, AppError> {
        let key = format!("org:{}/tool:{}", org_id, tool_id);

        if let Some(instance) = self.containers.get(&key) {
            if instance.info.status == SandboxStatus::Ready {
                return Ok(instance.info.container_id.clone());
            }
        }

        self.create_org_tool_sandbox(org_id, tool_id).await
    }

    /// Get or create a sandbox for a platform tool
    async fn get_or_create_platform_sandbox(
        &self,
        tool_id: &str,
        image: &str,
    ) -> Result<String, AppError> {
        let key = format!("platform:{}", tool_id);

        if let Some(instance) = self.containers.get(&key) {
            if instance.info.status == SandboxStatus::Ready {
                return Ok(instance.info.container_id.clone());
            }
        }

        self.create_platform_tool_sandbox(tool_id, image).await
    }

    /// Create a new sandbox for an org tool
    async fn create_org_tool_sandbox(
        &self,
        org_id: &str,
        tool_id: &str,
    ) -> Result<String, AppError> {
        let image = format!("ghcr.io/{}/{}:latest", org_id, tool_id);
        let key = format!("org:{}/tool:{}", org_id, tool_id);
        self.create_sandbox(&key, &image, Some(org_id.to_string()))
            .await
    }

    /// Create a new sandbox for a platform tool
    async fn create_platform_tool_sandbox(
        &self,
        tool_id: &str,
        image: &str,
    ) -> Result<String, AppError> {
        let key = format!("platform:{}", tool_id);
        self.create_sandbox(&key, image, None).await
    }

    /// Create a sandbox container
    async fn create_sandbox(
        &self,
        key: &str,
        image: &str,
        org_id: Option<String>,
    ) -> Result<String, AppError> {
        tracing::info!("Creating sandbox for {} with image {}", key, image);

        self.ensure_image(image).await?;

        let container_name = format!("aion-hive-{}", uuid::Uuid::new_v4());
        let session_id = org_id.unwrap_or_else(|| "global".to_string());
        let env_mode = "AION_TOOL_MODE=container".to_string();
        let env_session = format!("AION_SESSION_ID={}", session_id);
        let network_mode = "aion-hive-isolation".to_string();

        let config = Config {
            image: Some(image.to_string()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            env: Some(vec![env_mode, env_session]),
            cmd: Some(vec!["sleep".to_string(), "3600".to_string()]),
            host_config: Some(bollard::service::HostConfig {
                network_mode: Some(network_mode),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: &container_name,
            platform: None,
        };

        let response = self
            .docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create container: {}", e)))?;

        let container_id = response.id;

        self.docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to start container: {}", e)))?;

        let info = SandboxInfo {
            id: key.to_string(),
            session_id,
            container_id: container_id.clone(),
            image: image.to_string(),
            status: SandboxStatus::Ready,
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
        };

        self.containers
            .insert(key.to_string(), SandboxInstance { info });

        tracing::info!("Sandbox created: container_id={}", container_id);

        Ok(container_id)
    }

    /// Ensure the image exists locally, pull if not
    async fn ensure_image(&self, image: &str) -> Result<(), AppError> {
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![image.to_string()]);

        let options = ListImagesOptions::<String> {
            filters,
            ..Default::default()
        };

        let images = self
            .docker
            .list_images(Some(options))
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list images: {}", e)))?;

        if images.is_empty() {
            tracing::info!("Image {} not found locally, pulling...", image);
            let mut stream = self.docker.create_image(
                Some(bollard::image::CreateImageOptions {
                    from_image: image,
                    ..Default::default()
                }),
                None,
                None,
            );
            while let Some(result) = stream.next().await {
                if let Err(e) = result {
                    tracing::warn!("Pull image progress error: {}", e);
                }
            }
            tracing::info!("Image {} pulled successfully", image);
        }

        Ok(())
    }

    /// Execute a tool in a running container
    async fn execute_in_container(
        &self,
        container_id: &str,
        parameters: &HashMap<String, serde_json::Value>,
        _timeout_seconds: u64,
    ) -> Result<serde_json::Value, AppError> {
        let params_json = serde_json::to_string(parameters)
            .map_err(|e| AppError::ValidationError(format!("Failed to serialize params: {}", e)))?;

        let exec_cmd = format!("echo '{}' | tool-execute", params_json);
        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), exec_cmd]),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create exec: {}", e)))?;

        let output = match self.docker.start_exec(&exec.id, None).await {
            Ok(StartExecResults::Attached { mut output, .. }) => {
                let mut stdout = String::new();
                let mut stderr = String::new();

                use futures_util::StreamExt;
                while let Some(result) = output.next().await {
                    match result {
                        Ok(msg) => {
                            let line = msg.to_string();
                            if line.starts_with(&"stdout:") {
                                stdout.push_str(&line[7..]);
                            } else if line.starts_with(&"stderr:") {
                                stderr.push_str(&line[6..]);
                            } else {
                                stdout.push_str(&line);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Exec output error: {}", e);
                        }
                    }
                }
                (stdout, stderr)
            }
            Ok(StartExecResults::Detached) => {
                return Err(AppError::InternalError(
                    "Exec detached unexpectedly".to_string(),
                ));
            }
            Err(e) => {
                return Err(AppError::InternalError(format!(
                    "Failed to start exec: {}",
                    e
                )));
            }
        };

        if !output.1.is_empty() {
            tracing::warn!("stderr from container: {}", output.1);
        }

        let result: serde_json::Value = serde_json::from_str(&output.0.trim()).map_err(|e| {
            AppError::ValidationError(format!(
                "Failed to parse output: {} (output: {})",
                e, output.0
            ))
        })?;

        Ok(result)
    }

    /// Execute with custom timeout
    pub async fn execute_with_timeout(
        &self,
        request: ToolExecutionRequest,
        timeout_seconds: u64,
    ) -> Result<ToolExecutionResult, AppError> {
        if timeout_seconds == 0 {
            return Err(AppError::ValidationError("Timeout must be > 0".to_string()));
        }

        self.execute_org_tool(request).await
    }

    /// List available sandbox images for an org
    pub async fn list_sandbox_images(&self, _org_id: &str) -> Result<Vec<String>, AppError> {
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec!["aion-hive/*".to_string()]);

        let options = ListImagesOptions::<String> {
            filters,
            ..Default::default()
        };

        let images = self
            .docker
            .list_images(Some(options))
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list images: {}", e)))?;

        Ok(images
            .iter()
            .map(|img| {
                img.repo_tags
                    .first()
                    .cloned()
                    .unwrap_or_else(|| img.id.clone())
            })
            .collect())
    }

    /// List active containers
    pub async fn list_containers(&self) -> Result<Vec<SandboxInfo>, AppError> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec!["aion-hive-*".to_string()]);

        let options = ListContainersOptions::<String> {
            filters,
            all: true,
            ..Default::default()
        };

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list containers: {}", e)))?;

        Ok(containers
            .into_iter()
            .map(|c| SandboxInfo {
                id: c.names.and_then(|n| n.first().cloned()).unwrap_or_default(),
                session_id: c
                    .labels
                    .and_then(|l| l.get("session_id").cloned())
                    .unwrap_or_default(),
                container_id: c.id.unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                status: match c.state.as_deref() {
                    Some("running") => SandboxStatus::Ready,
                    Some("exited") => SandboxStatus::Stopped,
                    _ => SandboxStatus::Stopped,
                },
                created_at: chrono::DateTime::from_timestamp(c.created.unwrap_or(0), 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
                last_used: chrono::Utc::now(),
            })
            .collect())
    }

    /// Stop and remove a sandbox
    pub async fn remove_sandbox(&self, key: &str) -> Result<(), AppError> {
        if let Some((_key, instance)) = self.containers.remove(key) {
            let container_id = &instance.info.container_id;

            let _ = self.docker.stop_container(container_id, None).await;
            let options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            self.docker
                .remove_container(container_id, Some(options))
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to remove container: {}", e))
                })?;

            tracing::info!("Sandbox {} removed", key);
        }
        Ok(())
    }

    /// Cleanup old containers
    pub async fn cleanup_stale_containers(&self, max_age_seconds: u64) -> Result<usize, AppError> {
        let now = chrono::Utc::now().timestamp();
        let mut removed = 0;

        for entry in self.containers.iter() {
            let age = now - entry.value().info.created_at.timestamp();
            if age > max_age_seconds as i64 {
                if let Err(e) = self.remove_sandbox(entry.key()).await {
                    tracing::warn!("Failed to remove stale sandbox {}: {}", entry.key(), e);
                } else {
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }

    /// Health check - verify Docker daemon is accessible
    pub async fn health_check(&self) -> Result<bool, AppError> {
        self.docker
            .ping()
            .await
            .map_err(|e| AppError::InternalError(format!("Docker health check failed: {}", e)))?;
        Ok(true)
    }

    /// Create network for sandbox isolation
    pub async fn ensure_isolation_network(&self) -> Result<(), AppError> {
        use bollard::network::CreateNetworkOptions;

        let networks = self
            .docker
            .list_networks::<String>(None)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list networks: {}", e)))?;

        let network_exists = networks
            .iter()
            .any(|n| n.name.as_deref() == Some("aion-hive-isolation"));

        if !network_exists {
            let config = CreateNetworkOptions {
                name: "aion-hive-isolation",
                driver: "bridge",
                internal: true,
                ..Default::default()
            };

            self.docker.create_network(config).await.map_err(|e| {
                AppError::InternalError(format!("Failed to create isolation network: {}", e))
            })?;

            tracing::info!("Created isolation network: aion-hive-isolation");
        }

        Ok(())
    }
}

impl Default for SandboxService {
    fn default() -> Self {
        Self::new()
    }
}
