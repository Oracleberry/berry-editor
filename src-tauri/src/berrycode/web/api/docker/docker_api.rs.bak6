use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "web")]
use bollard::{
    Docker,
    container::{
        ListContainersOptions, StartContainerOptions, StopContainerOptions,
        RemoveContainerOptions, LogsOptions, Stats, StatsOptions,
    },
    image::{ListImagesOptions, RemoveImageOptions},
    network::ListNetworksOptions,
    volume::ListVolumesOptions,
};

/// Docker API State
#[derive(Clone)]
pub struct DockerApiState {
    docker: Docker,
}

impl DockerApiState {
    pub fn new() -> anyhow::Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    /// Create a dummy DockerApiState for when Docker daemon is not available
    /// This prevents startup failures but Docker features will not work
    pub fn new_dummy() -> Self {
        // Try multiple connection methods in order of preference
        // Create a Docker client that may not connect but won't panic
        let docker =
            Docker::connect_with_unix_defaults().ok()
            .or_else(|| Docker::connect_with_local_defaults().ok())
            .or_else(|| Docker::connect_with_socket_defaults().ok())
            .or_else(|| Docker::connect_with_http_defaults().ok())
            .expect("Failed to create any Docker connection - this should never happen");
        Self { docker }
    }
}

/// Query params for session
#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub session_id: String,
}

// ===== Container Models =====

#[derive(Debug, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: i64,
    pub ports: Vec<PortMapping>,
}

#[derive(Debug, Serialize)]
pub struct PortMapping {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub port_type: String,
}

#[derive(Debug, Deserialize)]
pub struct ContainerActionRequest {
    pub container_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ContainerLogsQuery {
    pub session_id: String,
    pub container_id: String,
    pub tail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: f64,
    pub network_rx: u64,
    pub network_tx: u64,
}

// ===== Image Models =====

#[derive(Debug, Serialize)]
pub struct ImageInfo {
    pub id: String,
    pub tags: Vec<String>,
    pub size: i64,
    pub created: i64,
}

#[derive(Debug, Deserialize)]
pub struct ImageActionRequest {
    pub image_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ImagePullRequest {
    pub image_name: String,
}

// ===== Network Models =====

#[derive(Debug, Serialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

// ===== Volume Models =====

#[derive(Debug, Serialize)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created: String,
}

// ===== API Handlers =====

/// List all containers
pub async fn list_containers(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<Vec<ContainerInfo>>, StatusCode> {
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });

    let containers = state.docker
        .list_containers(options)
        .await
        .map_err(|e| {
            eprintln!("Failed to list containers: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let container_infos = containers
        .into_iter()
        .map(|c| {
            let ports = c.ports.unwrap_or_default()
                .into_iter()
                .map(|p| PortMapping {
                    private_port: p.private_port,
                    public_port: p.public_port,
                    port_type: p.typ.map(|t| format!("{:?}", t)).unwrap_or_else(|| "tcp".to_string()),
                })
                .collect();

            ContainerInfo {
                id: c.id.unwrap_or_default(),
                name: c.names.unwrap_or_default().first().cloned().unwrap_or_default().trim_start_matches('/').to_string(),
                image: c.image.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                created: c.created.unwrap_or_default(),
                ports,
            }
        })
        .collect();

    Ok(Json(container_infos))
}

/// Start a container
pub async fn start_container(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
    Json(payload): Json<ContainerActionRequest>,
) -> Result<Json<String>, StatusCode> {
    state.docker
        .start_container(&payload.container_id, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| {
            eprintln!("Failed to start container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(format!("Container {} started", payload.container_id)))
}

/// Stop a container
pub async fn stop_container(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
    Json(payload): Json<ContainerActionRequest>,
) -> Result<Json<String>, StatusCode> {
    state.docker
        .stop_container(&payload.container_id, None)
        .await
        .map_err(|e| {
            eprintln!("Failed to stop container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(format!("Container {} stopped", payload.container_id)))
}

/// Restart a container
pub async fn restart_container(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
    Json(payload): Json<ContainerActionRequest>,
) -> Result<Json<String>, StatusCode> {
    state.docker
        .restart_container(&payload.container_id, None)
        .await
        .map_err(|e| {
            eprintln!("Failed to restart container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(format!("Container {} restarted", payload.container_id)))
}

/// Remove a container
pub async fn remove_container(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
    Json(payload): Json<ContainerActionRequest>,
) -> Result<Json<String>, StatusCode> {
    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });

    state.docker
        .remove_container(&payload.container_id, options)
        .await
        .map_err(|e| {
            eprintln!("Failed to remove container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(format!("Container {} removed", payload.container_id)))
}

/// Get container logs
pub async fn get_container_logs(
    Query(query): Query<ContainerLogsQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<String>, StatusCode> {
    use futures::stream::StreamExt;

    let options = Some(LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: query.tail.unwrap_or_else(|| "100".to_string()),
        ..Default::default()
    });

    let mut logs = state.docker.logs(&query.container_id, options);
    let mut output = String::new();

    while let Some(log) = logs.next().await {
        match log {
            Ok(log_line) => {
                output.push_str(&log_line.to_string());
            }
            Err(e) => {
                eprintln!("Error reading logs: {}", e);
                break;
            }
        }
    }

    Ok(Json(output))
}

/// Get container stats
pub async fn get_container_stats(
    Query(query): Query<ContainerLogsQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<ContainerStats>, StatusCode> {
    use futures::stream::StreamExt;

    let options = Some(StatsOptions {
        stream: false,
        one_shot: true,
    });

    let mut stats_stream = state.docker.stats(&query.container_id, options);

    if let Some(stats_result) = stats_stream.next().await {
        let stats = stats_result.map_err(|e| {
            eprintln!("Failed to get stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Calculate CPU percentage
        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage -
            stats.precpu_stats.cpu_usage.total_usage;
        let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) -
            stats.precpu_stats.system_cpu_usage.unwrap_or(0);
        let cpu_percent = if system_delta > 0 {
            (cpu_delta as f64 / system_delta as f64) *
            stats.cpu_stats.online_cpus.unwrap_or(1) as f64 * 100.0
        } else {
            0.0
        };

        // Memory stats
        let memory_usage = stats.memory_stats.usage.unwrap_or(0);
        let memory_limit = stats.memory_stats.limit.unwrap_or(1);
        let memory_percent = (memory_usage as f64 / memory_limit as f64) * 100.0;

        // Network stats
        let (network_rx, network_tx) = stats.networks
            .map(|networks| {
                networks.values().fold((0u64, 0u64), |(rx, tx), net| {
                    (rx + net.rx_bytes, tx + net.tx_bytes)
                })
            })
            .unwrap_or((0, 0));

        Ok(Json(ContainerStats {
            cpu_percent,
            memory_usage,
            memory_limit,
            memory_percent,
            network_rx,
            network_tx,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// List all images
pub async fn list_images(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<Vec<ImageInfo>>, StatusCode> {
    let options = Some(ListImagesOptions::<String> {
        all: true,
        ..Default::default()
    });

    let images = state.docker
        .list_images(options)
        .await
        .map_err(|e| {
            eprintln!("Failed to list images: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let image_infos = images
        .into_iter()
        .map(|img| ImageInfo {
            id: img.id,
            tags: img.repo_tags,
            size: img.size,
            created: img.created,
        })
        .collect();

    Ok(Json(image_infos))
}

/// Remove an image
pub async fn remove_image(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
    Json(payload): Json<ImageActionRequest>,
) -> Result<Json<String>, StatusCode> {
    let options = Some(RemoveImageOptions {
        force: true,
        ..Default::default()
    });

    state.docker
        .remove_image(&payload.image_id, options, None)
        .await
        .map_err(|e| {
            eprintln!("Failed to remove image: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(format!("Image {} removed", payload.image_id)))
}

/// Pull an image
pub async fn pull_image(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
    Json(payload): Json<ImagePullRequest>,
) -> Result<Json<String>, StatusCode> {
    use bollard::image::CreateImageOptions;
    use futures::stream::StreamExt;

    let options = Some(CreateImageOptions {
        from_image: payload.image_name.clone(),
        ..Default::default()
    });

    let mut stream = state.docker.create_image(options, None, None);

    while let Some(result) = stream.next().await {
        match result {
            Ok(_info) => {
                // Progress info
            }
            Err(e) => {
                eprintln!("Failed to pull image: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(Json(format!("Image {} pulled successfully", payload.image_name)))
}

/// List all networks
pub async fn list_networks(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<Vec<NetworkInfo>>, StatusCode> {
    let options = Some(ListNetworksOptions::<String> {
        ..Default::default()
    });

    let networks = state.docker
        .list_networks(options)
        .await
        .map_err(|e| {
            eprintln!("Failed to list networks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let network_infos = networks
        .into_iter()
        .map(|net| NetworkInfo {
            id: net.id.unwrap_or_default(),
            name: net.name.unwrap_or_default(),
            driver: net.driver.unwrap_or_default(),
            scope: net.scope.unwrap_or_default(),
        })
        .collect();

    Ok(Json(network_infos))
}

/// List all volumes
pub async fn list_volumes(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<Vec<VolumeInfo>>, StatusCode> {
    let options = Some(ListVolumesOptions::<String> {
        ..Default::default()
    });

    let volumes_response = state.docker
        .list_volumes(options)
        .await
        .map_err(|e| {
            eprintln!("Failed to list volumes: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let volume_infos = volumes_response.volumes
        .unwrap_or_default()
        .into_iter()
        .map(|vol| VolumeInfo {
            name: vol.name,
            driver: vol.driver,
            mountpoint: vol.mountpoint,
            created: vol.created_at.unwrap_or_default(),
        })
        .collect();

    Ok(Json(volume_infos))
}

/// Get Docker system info
pub async fn get_docker_info(
    Query(_query): Query<SessionQuery>,
    State(state): State<DockerApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let info = state.docker
        .info()
        .await
        .map_err(|e| {
            eprintln!("Failed to get Docker info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::to_value(info).unwrap_or_default()))
}
