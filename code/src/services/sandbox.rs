//! Sandbox Service - Executes Org CLI tools in isolated Docker containers
//!
//! This service provides isolated execution environment for org tools and platform tools.
//! Tools are packaged as Docker images and executed via bollard SDK.
//!
//! Docker connection: TCP socket (default: tcp://localhost:2375)

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};
use tokio::time::timeout as tokio_timeout;

use crate::models::error::AppError;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub docker_host: String,
    pub default_timeout: u64,
    /// Maximum time (seconds) a container can exist before being reclaimed
    pub max_container_lifetime: u64,
    /// Maximum idle time (seconds) before an unused container is released
    pub max_idle_seconds: u64,
    /// Maximum number of concurrent sandbox containers across all tools
    pub max_containers: usize,
    /// Maximum number of pooled containers per individual tool
    pub max_per_tool: usize,
    /// Maximum time (seconds) a request waits in the per-tool FIFO queue
    /// before failing with a "queue full" error
    pub max_queue_wait_seconds: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            docker_host: std::env::var("DOCKER_HOST")
                .unwrap_or_else(|_| "http://localhost:2375".to_string()),
            default_timeout: 30,
            max_container_lifetime: 3600, // 1 hour total lifetime
            max_idle_seconds: 600,        // 10 minutes idle → release
            max_containers: 50,           // max 50 concurrent containers (global)
            max_per_tool: 5,              // max 5 pooled containers per tool
            max_queue_wait_seconds: 60,   // wait up to 60s in the queue
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
    /// Optional custom Docker image; falls back to ghcr.io/{org_id}/{tool_id}:latest
    #[serde(default)]
    pub docker_image: Option<String>,
    /// Optional session ID for execution history recording
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optional custom command to run inside the container (e.g., ["gh", "issue", "list"])
    /// Falls back to "tool-execute" if not specified
    #[serde(default)]
    pub cmd: Option<Vec<String>>,
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
    /// Logical tool key (e.g. `org:acme/tool:issue_lister`). Multiple pooled
    /// containers can share the same key.
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

/// Per-tool container pool.
///
/// - `sem` caps concurrent executions for this tool and provides a **FIFO
///   fair queue** (tokio `Semaphore` is first-in-first-out) so that a burst
///   of requests is served in arrival order rather than starving.
/// - `avail` holds idle (Ready) container IDs that can be reused immediately.
/// - `total` is the number of live containers owned by this pool.
/// - `notify` wakes a queued waiter when a container becomes available.
#[derive(Debug)]
struct ToolPool {
    sem: Arc<Semaphore>,
    avail: Mutex<Vec<String>>,
    total: AtomicUsize,
    notify: Notify,
}

/// RAII lease for an acquired container. Holding it reserves a per-tool
/// semaphore permit (concurrency slot). On drop the permit is released.
/// The container is returned to the pool by `return_container`.
struct ContainerLease {
    container_id: String,
    pool: Arc<ToolPool>,
    _permit: OwnedSemaphorePermit,
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
    /// Global registry of all live containers, keyed by container_id.
    containers: Arc<DashMap<String, SandboxInstance>>,
    /// Per-tool pools, keyed by logical tool key.
    pools: Arc<DashMap<String, Arc<ToolPool>>>,
    platform_tools: Arc<HashMap<String, PlatformTool>>,
    default_timeout: u64,
    max_container_lifetime: u64,
    max_idle_seconds: u64,
    max_containers: usize,
    max_per_tool: usize,
    max_queue_wait_seconds: u64,
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
                tracing::warn!("Docker not available ({}), sandbox features disabled", e);
                None
            });

        Self {
            docker,
            containers: Arc::new(DashMap::new()),
            pools: Arc::new(DashMap::new()),
            platform_tools: Arc::new(platform_tools),
            default_timeout: config.default_timeout,
            max_container_lifetime: config.max_container_lifetime,
            max_idle_seconds: config.max_idle_seconds,
            max_containers: config.max_containers,
            max_per_tool: config.max_per_tool,
            max_queue_wait_seconds: config.max_queue_wait_seconds,
        }
    }

    /// Check whether Docker is available for sandbox operations.
    pub fn is_available(&self) -> bool {
        self.docker.is_some()
    }

    /// Get a reference to the Docker client, or return an error if unavailable.
    fn docker(&self) -> Result<&Docker, AppError> {
        self.docker.as_ref().ok_or_else(|| {
            AppError::InternalError(
                "Docker is not available. Please start Docker daemon to enable sandbox features."
                    .to_string(),
            )
        })
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

    /// Start a background task that periodically removes stale and idle containers.
    fn start_cleanup_task(&self) {
        let pools = self.pools.clone();
        let containers = self.containers.clone();
        let docker = match &self.docker {
            Some(d) => d.clone(),
            None => return,
        };
        let max_lifetime = self.max_container_lifetime;
        let max_idle = self.max_idle_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // every 1 min
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                let mut to_remove: Vec<(String, Arc<ToolPool>)> = Vec::new();

                // Only idle (available) containers are eligible for eviction, so we
                // never kill a container that is currently executing.
                for pool_entry in pools.iter() {
                    let pool = pool_entry.value().clone();
                    let avail = pool.avail.lock().unwrap();
                    for cid in avail.iter() {
                        if let Some(inst) = containers.get(cid) {
                            let age = now - inst.value().info.created_at.timestamp();
                            let idle = now - inst.value().info.last_used.timestamp();
                            if age > max_lifetime as i64 || idle > max_idle as i64 {
                                to_remove.push((cid.clone(), pool.clone()));
                            }
                        }
                    }
                }

                for (cid, pool) in to_remove {
                    pool.avail.lock().unwrap().retain(|c| c != &cid);
                    pool.total.fetch_sub(1, Ordering::SeqCst);
                    if let Some((_k, inst)) = containers.remove(&cid) {
                        let _ = docker.stop_container(&inst.info.container_id, None).await;
                        let opts = RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        };
                        let _ = docker
                            .remove_container(&inst.info.container_id, Some(opts))
                            .await;
                        tracing::info!("Cleaned up sandbox container: {}", cid);
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
            Ok(containers) => containers
                .iter()
                .any(|c| c.state.as_deref() == Some("running")),
            Err(_) => false,
        }
    }

    /// Acquire a container lease for the given tool key.
    ///
    /// This is the heart of the concurrency model:
    /// 1. Acquire a per-tool `Semaphore` permit. Because tokio semaphores are
    ///    FIFO, this forms a **fair queue** — excess requests wait their turn
    ///    instead of contending for a single container. A timeout bounds the wait.
    /// 2. Reuse an idle pooled container if one is available (health-checked).
    /// 3. Otherwise create a new container, respecting the per-tool limit
    ///    (`max_per_tool`) and the global limit (`max_containers`).
    /// 4. If the pool is at capacity and all containers are busy, block on a
    ///    `Notify` until a container is returned by another execution.
    async fn acquire_container(
        &self,
        tool_key: &str,
        image: &str,
        org_id: Option<String>,
        queue_timeout: u64,
    ) -> Result<ContainerLease, AppError> {
        let pool = self.get_or_create_pool(tool_key);

        // 1. Fair FIFO queue via per-tool semaphore, with bounded wait.
        let permit = tokio_timeout(
            Duration::from_secs(queue_timeout),
            pool.sem.clone().acquire_owned(),
        )
        .await
        .map_err(|_| {
            AppError::InternalError(format!(
                "Tool '{}' execution queue is full: timed out after {}s waiting for a free slot",
                tool_key, queue_timeout
            ))
        })?
        .map_err(|e| AppError::InternalError(format!("Sandbox semaphore closed: {}", e)))?;

        loop {
            // 2. Fast path: reuse an idle, healthy container.
            let reuse = {
                let mut avail = pool.avail.lock().unwrap();
                avail.pop()
            };
            if let Some(cid) = reuse {
                if self.is_container_running(&cid).await
                    && self.health_check_container(&cid).await.unwrap_or(false)
                {
                    self.mark_busy(&cid);
                    return Ok(ContainerLease {
                        container_id: cid,
                        pool: pool.clone(),
                        _permit: permit,
                    });
                }
                // Unhealthy: drop it and retry (decrement pool total).
                self.remove_container_by_id(&cid, &pool).await;
                pool.total.fetch_sub(1, Ordering::SeqCst);
                continue;
            }

            // 3. Need a new container — allowed only if under per-tool limit.
            let can_create = pool.total.load(Ordering::SeqCst) < self.max_per_tool;
            if can_create {
                // Enforce global limit: evict the global LRU idle container first.
                if self.containers.len() >= self.max_containers {
                    self.evict_global_lru().await;
                }
                match self.create_sandbox(tool_key, image, org_id.clone()).await {
                    Ok(cid) => {
                        pool.total.fetch_add(1, Ordering::SeqCst);
                        self.mark_busy(&cid);
                        return Ok(ContainerLease {
                            container_id: cid,
                            pool: pool.clone(),
                            _permit: permit,
                        });
                    }
                    Err(e) => {
                        pool.total.fetch_sub(1, Ordering::SeqCst);
                        return Err(e);
                    }
                }
            }

            // 4. Pool at capacity & all busy → wait for one to free up.
            pool.notify.notified().await;
        }
    }

    /// Return a leased container to its pool, marking it Ready and waking a waiter.
    async fn return_container(&self, lease: ContainerLease) {
        let cid = lease.container_id.clone();
        if let Some(mut inst) = self.containers.get_mut(&cid) {
            inst.info.status = SandboxStatus::Ready;
            inst.info.last_used = chrono::Utc::now();
        }
        lease.pool.avail.lock().unwrap().push(cid);
        lease.pool.notify.notify_one();
        // `permit` is released when `lease` is dropped here.
    }

    /// Get (or lazily create) the per-tool pool.
    fn get_or_create_pool(&self, tool_key: &str) -> Arc<ToolPool> {
        self.pools
            .entry(tool_key.to_string())
            .or_insert_with(|| {
                Arc::new(ToolPool {
                    sem: Arc::new(Semaphore::new(self.max_per_tool)),
                    avail: Mutex::new(Vec::new()),
                    total: AtomicUsize::new(0),
                    notify: Notify::new(),
                })
            })
            .clone()
    }

    /// Execute an org tool in a sandboxed Docker container.
    ///
    /// Concurrency and queueing are handled transparently by `acquire_container`,
    /// so the MCP/HTTP caller does not need to be aware of pooling.
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
        let image = request
            .docker_image
            .clone()
            .unwrap_or_else(|| format!("ghcr.io/{}/{}:latest", request.org_id, request.tool_id));

        let lease = self
            .acquire_container(
                &key,
                &image,
                Some(request.org_id.clone()),
                self.max_queue_wait_seconds,
            )
            .await?;

        let result = self
            .execute_in_container(
                &lease.container_id,
                &request.tool_id,
                &request.parameters,
                timeout,
                request.cmd.as_deref(),
            )
            .await;

        let execution_time_ms = start.elapsed().as_millis() as u64;
        self.return_container(lease).await;

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
        let lease = self
            .acquire_container(&key, &tool.image, None, self.max_queue_wait_seconds)
            .await?;

        let result = self
            .execute_in_container(&lease.container_id, tool_id, &parameters, timeout, None)
            .await;

        let execution_time_ms = start.elapsed().as_millis() as u64;
        self.return_container(lease).await;

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

    /// Quick health check: exec a lightweight command to see if container is responsive.
    async fn health_check_container(&self, container_id: &str) -> Result<bool, AppError> {
        let docker = match self.docker() {
            Ok(d) => d,
            Err(_) => return Ok(false),
        };

        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["echo".to_string(), "ok".to_string()]),
            ..Default::default()
        };

        let exec = match docker.create_exec(container_id, exec_config).await {
            Ok(e) => e,
            Err(_) => return Ok(false),
        };

        match docker.start_exec(&exec.id, None).await {
            Ok(StartExecResults::Attached { mut output, .. }) => {
                // Just consume output; any response means the container is alive
                while let Some(result) = output.next().await {
                    if result.is_err() {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Force-stop and remove a container (best-effort, ignores errors).
    async fn force_remove_container(&self, container_id: &str) {
        if let Ok(docker) = self.docker() {
            let _ = docker.stop_container(container_id, None).await;
            let opts = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            let _ = docker.remove_container(container_id, Some(opts)).await;
        }
    }

    /// Remove a single container from the registry and its owning pool.
    async fn remove_container_by_id(&self, cid: &str, pool: &Arc<ToolPool>) {
        pool.avail.lock().unwrap().retain(|c| c != cid);
        if let Some((_k, inst)) = self.containers.remove(cid) {
            let _ = self.force_remove_container(&inst.info.container_id).await;
        }
    }

    /// Evict the globally least-recently-used *idle* container to make room
    /// under the global `max_containers` cap.
    async fn evict_global_lru(&self) {
        let mut lru: Option<(String, Arc<ToolPool>, i64)> = None;

        for pool_entry in self.pools.iter() {
            let pool = pool_entry.value().clone();
            let avail = pool.avail.lock().unwrap();
            for cid in avail.iter() {
                if let Some(inst) = self.containers.get(cid) {
                    let ts = inst.value().info.last_used.timestamp();
                    if lru.as_ref().map(|(_, _, t)| ts < *t).unwrap_or(true) {
                        lru = Some((cid.clone(), pool.clone(), ts));
                    }
                }
            }
        }

        if let Some((cid, pool, _)) = lru {
            pool.avail.lock().unwrap().retain(|c| c != &cid);
            pool.total.fetch_sub(1, Ordering::SeqCst);
            if let Some((_k, inst)) = self.containers.remove(&cid) {
                let _ = self.force_remove_container(&inst.info.container_id).await;
                tracing::info!("Evicted global LRU sandbox container: {}", cid);
            }
        }
    }

    /// Mark a container as Busy in the registry.
    fn mark_busy(&self, cid: &str) {
        if let Some(mut inst) = self.containers.get_mut(cid) {
            inst.info.status = SandboxStatus::Busy;
        }
    }

    /// Release (stop + remove) a specific sandbox by org/tool key.
    /// Releases *all* pooled containers for that tool.
    pub async fn release_sandbox(&self, org_id: &str, tool_id: &str) -> Result<bool, AppError> {
        let key = format!("org:{}/tool:{}", org_id, tool_id);
        let mut released = false;

        if let Some(pool) = self.pools.get(&key).map(|p| p.clone()) {
            let cids: Vec<String> = {
                let mut avail = pool.avail.lock().unwrap();
                std::mem::take(&mut *avail)
            };
            for cid in cids {
                if let Some((_k, inst)) = self.containers.remove(&cid) {
                    let _ = self.force_remove_container(&inst.info.container_id).await;
                    released = true;
                }
            }
            pool.total.store(0, Ordering::SeqCst);
            self.pools.remove(&key);
        }

        Ok(released)
    }

    /// Current number of active sandbox containers.
    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    /// Max configured container limit (global).
    pub fn max_containers(&self) -> usize {
        self.max_containers
    }

    /// List all active sandbox info entries.
    pub fn list_active_sandboxes(&self) -> Vec<SandboxInfo> {
        self.containers
            .iter()
            .map(|e| e.value().info.clone())
            .collect()
    }

    /// Create a new sandbox container, registered under its container_id.
    async fn create_sandbox(
        &self,
        tool_key: &str,
        image: &str,
        org_id: Option<String>,
    ) -> Result<String, AppError> {
        tracing::info!("Creating sandbox for {} with image {}", tool_key, image);

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
            id: tool_key.to_string(),
            session_id,
            container_id: container_id.clone(),
            image: image.to_string(),
            status: SandboxStatus::Ready,
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
        };

        self.containers
            .insert(container_id.clone(), SandboxInstance { info });

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

    /// Execute a tool inside a running container via `docker exec`.
    ///
    /// `docker exec` spawns a **new** process inside the container, completely
    /// independent of the container's ENTRYPOINT / CMD.  The container only
    /// needs to stay alive (it runs `sleep 3600` on startup), so any image can
    /// serve as a sandbox regardless of its original entrypoint.
    ///
    /// Two execution modes:
    /// - **Custom cmd mode** (org tools with `implementation.cmd`):
    ///   Runs the user-specified binary/script, feeding JSON parameters via
    ///   stdin heredoc. No `_tool_id` is injected — the container image is
    ///   user-provided and has no concept of database-level tool identifiers.
    ///   Example: `python /app/tools/issue_lister.py <<'EOF'\n{...}\nEOF`
    /// - **Platform mode** (no custom cmd, uses `tool-execute`):
    ///   Uses the built-in `tool-execute` entrypoint. Injects `_tool_id` so
    ///   the platform script can route to the correct handler.
    async fn execute_in_container(
        &self,
        container_id: &str,
        tool_id: &str,
        parameters: &HashMap<String, serde_json::Value>,
        timeout_seconds: u64,
        cmd: Option<&[String]>,
    ) -> Result<serde_json::Value, AppError> {
        let mut params = parameters.clone();
        let delimiter = format!("EOF_{}", uuid::Uuid::new_v4().simple());

        let exec_cmd =
            if let Some(c) = cmd {
                // --- Org tool: custom command from implementation ---
                // Build a shell one-liner:  <cmd...> <<'EOF_xxx'\n<json>\nEOF_xxx
                // The container binary/script reads JSON params from stdin.
                // No _tool_id — the image has no knowledge of database IDs.
                let cmd_str = c.join(" ");
                format!(
                    "{} <<'{}'\n{}\n{}",
                    cmd_str,
                    delimiter,
                    serde_json::to_string(&params).map_err(|e| AppError::ValidationError(
                        format!("Failed to serialize params: {}", e)
                    ))?,
                    delimiter
                )
            } else {
                // --- Platform tool / default: built-in `tool-execute` ---
                params.insert(
                    "_tool_id".to_string(),
                    serde_json::Value::String(tool_id.to_string()),
                );
                format!(
                    "tool-execute <<'{}'\n{}\n{}",
                    delimiter,
                    serde_json::to_string(&params).map_err(|e| AppError::ValidationError(
                        format!("Failed to serialize params: {}", e)
                    ))?,
                    delimiter
                )
            };

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

        let result: serde_json::Value = serde_json::from_str(output.0.trim()).map_err(|e| {
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

    /// Stop and remove a sandbox container by its container_id.
    pub async fn remove_sandbox(&self, container_id: &str) -> Result<(), AppError> {
        // Find the owning pool and detach from its avail list.
        for pool_entry in self.pools.iter() {
            let pool = pool_entry.value().clone();
            let mut avail = pool.avail.lock().unwrap();
            if avail.iter().any(|c| c == container_id) {
                avail.retain(|c| c != container_id);
                pool.total.fetch_sub(1, Ordering::SeqCst);
                drop(avail);
                break;
            }
        }
        if let Some((_key, instance)) = self.containers.remove(container_id) {
            self.force_remove_container(&instance.info.container_id)
                .await;
        }
        Ok(())
    }

    /// Cleanup old containers by age.
    pub async fn cleanup_stale_containers(&self, max_age_seconds: u64) -> Result<usize, AppError> {
        let now = chrono::Utc::now().timestamp();
        let mut removed = 0;

        let stale: Vec<String> = self
            .containers
            .iter()
            .filter(|e| now - e.value().info.created_at.timestamp() > max_age_seconds as i64)
            .map(|e| e.key().clone())
            .collect();

        for cid in stale {
            if let Err(e) = self.remove_sandbox(&cid).await {
                tracing::warn!("Failed to remove stale sandbox {}: {}", cid, e);
            } else {
                removed += 1;
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
