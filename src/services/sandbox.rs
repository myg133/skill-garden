//! Sandbox Service - Executes Org CLI tools in isolated Docker containers
//!
//! This service provides isolated execution environment for org tools and platform tools.
//! Tools are packaged as Docker images and executed via bollard SDK.
//!
//! Docker connection: TCP socket (default: tcp://localhost:2375)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
use tokio::time::timeout as tokio_timeout;

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
///
/// When Docker is not available, the service degrades gracefully:
/// all sandbox operations return an error indicating Docker is unavailable
/// but the server continues to run normally.
#[derive(Debug, Clone)]
pub struct SandboxService {
    docker: Option<Docker>,
    containers: Arc<DashMap<String, SandboxInstance>>,
    platform_tools: Arc<HashMap<String, PlatformTool>>,
    default_timeout: u64,
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

        // connect_with_local_defaults() reads DOCKER_HOST env var,
        // falling back to unix socket / named pipe.
        // If Docker is not available, store None so the server can
        // still start (sandbox operations will return clear errors).
        let docker = Docker::connect_with_local_defaults()
            .map(Some)
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Docker not available ({}), sandbox features disabled",
                    e
                );
                None
            });

        Self {
            docker,
            containers: Arc::new(DashMap::new()),
            platform_tools: Arc::new(platform_tools),
            default_timeout: config.default_timeout,
            max_container_lifetime: config.max_container_lifetime,
        }
    }

    /// Check whether Docker is available for sandbox operations.
    pub fn is_available(&self) -> bool {
        self.docker.is_some()
    }

    /// Get a reference to the Docker client, or return an error if unavailable.
    fn docker(&self) -> Result<&Docker, AppError> {
        self.docker
            .as_ref()
            .ok_or_else(|| AppError::InternalError("Docker is not available. Please start Docker daemon to enable sandbox features.".to_string()))
    }

    /// Initialize sandbox service: create isolation network + start cleanup background task.
    /// Does NOT fail if Docker is unavailable – logs a warning and continues.
    pub async fn initialize(&self) -> Result<(), AppError> {
        let docker = match &self.docker {
            Some(d) => d,
            None => {
                tracing::warn!("Sandbox service initialized without Docker – sandbox features will be unavailable");
                return Ok(());
            }
        };

        // Verify Docker daemon is actually reachable (connect_with_local_defaults
        // only creates a client struct, it doesn't test connectivity).
        if let Err(e) = docker.ping().await {
            tracing::warn!(
                "Docker daemon not reachable ({}). Sandbox features will be unavailable.",
                e
            );
            return Ok(());
        }

        if let Err(e) = self.ensure_isolation_network().await {
            tracing::warn!(
                "Failed to create isolation network: {}. Sandbox features will be unavailable.",
                e
            );
            return Ok(());
        }

        self.start_cleanup_task();
        Ok(())
    }

    /// Start a background task that periodically removes stale containers.
    fn start_cleanup_task(&self) {
        let containers = self.containers.clone();
        let docker = match &self.docker {
            Some(d) => d.clone(),
            None => return, // No Docker available, nothing to clean up
        };
        let max_lifetime = self.max_container_lifetime;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // every 5 min
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                let mut stale_keys: Vec<String> = Vec::new();

                for entry in containers.iter() {
                    let age = now - entry.value().info.created_at.timestamp();
                    if age > max_lifetime as i64 {
                        stale_keys.push(entry.key().clone());
                    }
                }

                for key in stale_keys {
                    if let Some((_k, instance)) = containers.remove(&key) {
                        let cid = instance.info.container_id;
                        let _ = docker.stop_container(&cid, None).await;
                        let opts = RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        };
                        let _ = docker.remove_container(&cid, Some(opts)).await;
                        tracing::info!("Cleaned up stale sandbox: {} (container={})", key, cid);
                    }
                }
            }
        });
    }

    /// Check if a Docker container is actually running.
    async fn is_container_running(&self, container_id: &str) -> bool {
        let docker = match self.docker() {
            Ok(d) => d,
            Err(_) => return false,
        };
        let mut filters = HashMap::new();
        filters.insert("id".to_string(), vec![container_id.to_string()]);
        let opts = ListContainersOptions::<String> {
            filters,
            all: true,
            ..Default::default()
        };
        match docker.list_containers(Some(opts)).await {
            Ok(containers) => containers.iter().any(|c| c.state.as_deref() == Some("running")),
            Err(_) => false,
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

        let key = format!("org:{}/tool:{}", request.org_id, request.tool_id);
        let sandbox = self
            .get_or_create_sandbox(&request.org_id, &request.tool_id)
            .await?;

        // Mark container as busy
        if let Some(mut instance) = self.containers.get_mut(&key) {
            instance.info.status = SandboxStatus::Busy;
        }

        let result = self
            .execute_in_container(&sandbox, &request.parameters, timeout)
            .await;

        // Mark container back to ready
        if let Some(mut instance) = self.containers.get_mut(&key) {
            instance.info.status = SandboxStatus::Ready;
            instance.info.last_used = chrono::Utc::now();
        }

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

        let key = format!("platform:{}", tool_id);
        let sandbox = self
            .get_or_create_platform_sandbox(tool_id, &tool.image)
            .await?;

        // Mark container as busy
        if let Some(mut instance) = self.containers.get_mut(&key) {
            instance.info.status = SandboxStatus::Busy;
        }

        let result = self
            .execute_in_container(&sandbox, &parameters, timeout)
            .await;

        // Mark container back to ready
        if let Some(mut instance) = self.containers.get_mut(&key) {
            instance.info.status = SandboxStatus::Ready;
            instance.info.last_used = chrono::Utc::now();
        }

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

        let reusable = if let Some(instance) = self.containers.get(&key) {
            instance.info.status == SandboxStatus::Ready
                && self.is_container_running(&instance.info.container_id).await
        } else {
            false
        };

        if reusable {
            if let Some(instance) = self.containers.get(&key) {
                return Ok(instance.info.container_id.clone());
            }
        } else {
            // Remove stale entry so we don't leak dead containers in the map
            self.containers.remove(&key);
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

        let reusable = if let Some(instance) = self.containers.get(&key) {
            instance.info.status == SandboxStatus::Ready
                && self.is_container_running(&instance.info.container_id).await
        } else {
            false
        };

        if reusable {
            if let Some(instance) = self.containers.get(&key) {
                return Ok(instance.info.container_id.clone());
            }
        } else {
            self.containers.remove(&key);
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

        let docker = self.docker()?;

        let response = docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create container: {}", e)))?;

        let container_id = response.id;

        docker
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

        let docker = self.docker()?;
        let images = docker
            .list_images(Some(options))
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list images: {}", e)))?;

        if images.is_empty() {
            tracing::info!("Image {} not found locally, pulling...", image);
            let mut stream = docker.create_image(
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
        timeout_seconds: u64,
    ) -> Result<serde_json::Value, AppError> {
        let params_json = serde_json::to_string(parameters)
            .map_err(|e| AppError::ValidationError(format!("Failed to serialize params: {}", e)))?;

        // Use heredoc with a UUID delimiter to safely pass JSON without shell injection
        let delimiter = format!("EOF_{}", uuid::Uuid::new_v4().simple());
        let exec_cmd = format!(
            "tool-execute <<'{}'\n{}\n{}",
            delimiter, params_json, delimiter
        );

        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), exec_cmd]),
            ..Default::default()
        };

        let docker = self.docker()?;
        let exec = docker
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create exec: {}", e)))?;

        let exec_future = async {
            match docker.start_exec(&exec.id, None).await {
                Ok(StartExecResults::Attached { mut output, .. }) => {
                    let mut stdout = String::new();
                    let mut stderr = String::new();

                    while let Some(result) = output.next().await {
                        match result {
                            Ok(msg) => {
                                let line = msg.to_string();
                                if line.starts_with("stdout:") {
                                    stdout.push_str(&line[7..]);
                                } else if line.starts_with("stderr:") {
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
                    Ok::<_, AppError>((stdout, stderr))
                }
                Ok(StartExecResults::Detached) => Err(AppError::InternalError(
                    "Exec detached unexpectedly".to_string(),
                )),
                Err(e) => Err(AppError::InternalError(format!(
                    "Failed to start exec: {}",
                    e
                ))),
            }
        };

        let output = tokio_timeout(Duration::from_secs(timeout_seconds), exec_future)
            .await
            .map_err(|_| {
                AppError::InternalError(format!(
                    "Tool execution timed out after {}s",
                    timeout_seconds
                ))
            })??;

        if !output.1.is_empty() {
            tracing::warn!("stderr from container: {}", output.1);
        }

        let result: serde_json::Value =
            serde_json::from_str(output.0.trim()).map_err(|e| {
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
        mut request: ToolExecutionRequest,
        timeout_seconds: u64,
    ) -> Result<ToolExecutionResult, AppError> {
        if timeout_seconds == 0 {
            return Err(AppError::ValidationError("Timeout must be > 0".to_string()));
        }

        request.timeout_seconds = timeout_seconds;
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

        let docker = self.docker()?;
        let images = docker
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

        let docker = self.docker()?;
        let containers = docker
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
        let docker = self.docker()?;
        if let Some((_key, instance)) = self.containers.remove(key) {
            let container_id = &instance.info.container_id;

            let _ = docker.stop_container(container_id, None).await;
            let options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            docker
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
        let docker = match self.docker() {
            Ok(d) => d,
            Err(_) => return Ok(false),
        };
        docker
            .ping()
            .await
            .map_err(|e| AppError::InternalError(format!("Docker health check failed: {}", e)))?;
        Ok(true)
    }

    /// Create network for sandbox isolation
    pub async fn ensure_isolation_network(&self) -> Result<(), AppError> {
        use bollard::network::CreateNetworkOptions;

        let docker = self.docker()?;
        let networks = docker
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

            docker.create_network(config).await.map_err(|e| {
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
