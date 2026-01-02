//! Kubernetes API for cluster visualization and management

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "web")]
use kube::{
    api::{Api, ListParams},
    Client,
};

#[cfg(feature = "web")]
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{Namespace, Pod, Service, ConfigMap, Secret, Node, PersistentVolumeClaim},
};

/// K8s API state
#[derive(Clone)]
pub struct K8sApiState {
    client: Arc<tokio::sync::RwLock<Option<Client>>>,
}

impl K8sApiState {
    pub fn new() -> Self {
        Self {
            client: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Initialize Kubernetes client
    pub async fn init_client(&self) -> Result<(), anyhow::Error> {
        #[cfg(feature = "web")]
        {
            // Try to create client from kubeconfig
            match Client::try_default().await {
                Ok(client) => {
                    *self.client.write().await = Some(client);
                    tracing::info!("✅ Kubernetes client initialized successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("⚠️  Failed to initialize Kubernetes client: {}", e);
                    tracing::info!("Kubernetes features will be disabled. Make sure kubectl is configured.");
                    Err(anyhow::anyhow!("Kubernetes client initialization failed: {}", e))
                }
            }
        }

        #[cfg(not(feature = "web"))]
        {
            Err(anyhow::anyhow!("Kubernetes features require 'web' feature"))
        }
    }

    pub async fn get_client(&self) -> Option<Client> {
        self.client.read().await.clone()
    }
}

/// Namespace information
#[derive(Debug, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub name: String,
    pub status: String,
    pub age: String,
}

/// Pod information
#[derive(Debug, Serialize, Deserialize)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub ready: String,
    pub restarts: i32,
    pub age: String,
    pub node: Option<String>,
}

/// Deployment information
#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub name: String,
    pub namespace: String,
    pub ready: String,
    pub up_to_date: i32,
    pub available: i32,
    pub age: String,
}

/// Service information
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub namespace: String,
    pub service_type: String,
    pub cluster_ip: String,
    pub external_ip: String,
    pub ports: String,
    pub age: String,
}

/// Node information
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub status: String,
    pub roles: String,
    pub age: String,
    pub version: String,
    pub os_image: String,
    pub kernel_version: String,
    pub container_runtime: String,
}

/// Cluster overview summary
#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterOverview {
    pub namespaces: usize,
    pub pods: usize,
    pub deployments: usize,
    pub services: usize,
    pub nodes: usize,
    pub healthy_pods: usize,
    pub unhealthy_pods: usize,
}

/// Query params
#[derive(Debug, Deserialize)]
pub struct NamespaceQuery {
    pub namespace: Option<String>,
}

/// Get all namespaces
pub async fn get_namespaces(
    State(state): State<K8sApiState>,
) -> Result<Json<Vec<NamespaceInfo>>, StatusCode> {
    #[cfg(feature = "web")]
    {
        let client = match state.get_client().await {
            Some(c) => c,
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

        let namespaces: Api<Namespace> = Api::all(client);

        match namespaces.list(&ListParams::default()).await {
            Ok(ns_list) => {
                let mut result = Vec::new();

                for ns in ns_list.items {
                    let name = ns.metadata.name.unwrap_or_default();
                    let status = ns
                        .status
                        .as_ref()
                        .and_then(|s| s.phase.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let age = ns
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| format_age(&t.0))
                        .unwrap_or_else(|| "Unknown".to_string());

                    result.push(NamespaceInfo { name, status, age });
                }

                Ok(Json(result))
            }
            Err(e) => {
                tracing::error!("Failed to list namespaces: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

/// Get all pods (optionally filtered by namespace)
pub async fn get_pods(
    Query(query): Query<NamespaceQuery>,
    State(state): State<K8sApiState>,
) -> Result<Json<Vec<PodInfo>>, StatusCode> {
    #[cfg(feature = "web")]
    {
        let client = match state.get_client().await {
            Some(c) => c,
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

        let pods: Api<Pod> = if let Some(ns) = query.namespace {
            Api::namespaced(client, &ns)
        } else {
            Api::all(client)
        };

        match pods.list(&ListParams::default()).await {
            Ok(pod_list) => {
                let mut result = Vec::new();

                for pod in pod_list.items {
                    let name = pod.metadata.name.unwrap_or_default();
                    let namespace = pod.metadata.namespace.unwrap_or_default();

                    let status = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.phase.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let ready = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.container_statuses.as_ref())
                        .map(|cs| {
                            let ready_count = cs.iter().filter(|c| c.ready).count();
                            format!("{}/{}", ready_count, cs.len())
                        })
                        .unwrap_or_else(|| "0/0".to_string());

                    let restarts = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.container_statuses.as_ref())
                        .map(|cs| cs.iter().map(|c| c.restart_count).sum())
                        .unwrap_or(0);

                    let age = pod
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| format_age(&t.0))
                        .unwrap_or_else(|| "Unknown".to_string());

                    let node = pod.spec.as_ref().and_then(|s| s.node_name.clone());

                    result.push(PodInfo {
                        name,
                        namespace,
                        status,
                        ready,
                        restarts,
                        age,
                        node,
                    });
                }

                Ok(Json(result))
            }
            Err(e) => {
                tracing::error!("Failed to list pods: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

/// Get all deployments
pub async fn get_deployments(
    Query(query): Query<NamespaceQuery>,
    State(state): State<K8sApiState>,
) -> Result<Json<Vec<DeploymentInfo>>, StatusCode> {
    #[cfg(feature = "web")]
    {
        let client = match state.get_client().await {
            Some(c) => c,
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

        let deployments: Api<Deployment> = if let Some(ns) = query.namespace {
            Api::namespaced(client, &ns)
        } else {
            Api::all(client)
        };

        match deployments.list(&ListParams::default()).await {
            Ok(deploy_list) => {
                let mut result = Vec::new();

                for deploy in deploy_list.items {
                    let name = deploy.metadata.name.unwrap_or_default();
                    let namespace = deploy.metadata.namespace.unwrap_or_default();

                    let ready = deploy
                        .status
                        .as_ref()
                        .map(|s| {
                            format!(
                                "{}/{}",
                                s.ready_replicas.unwrap_or(0),
                                s.replicas.unwrap_or(0)
                            )
                        })
                        .unwrap_or_else(|| "0/0".to_string());

                    let up_to_date = deploy
                        .status
                        .as_ref()
                        .and_then(|s| s.updated_replicas)
                        .unwrap_or(0);

                    let available = deploy
                        .status
                        .as_ref()
                        .and_then(|s| s.available_replicas)
                        .unwrap_or(0);

                    let age = deploy
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| format_age(&t.0))
                        .unwrap_or_else(|| "Unknown".to_string());

                    result.push(DeploymentInfo {
                        name,
                        namespace,
                        ready,
                        up_to_date,
                        available,
                        age,
                    });
                }

                Ok(Json(result))
            }
            Err(e) => {
                tracing::error!("Failed to list deployments: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

/// Get all services
pub async fn get_services(
    Query(query): Query<NamespaceQuery>,
    State(state): State<K8sApiState>,
) -> Result<Json<Vec<ServiceInfo>>, StatusCode> {
    #[cfg(feature = "web")]
    {
        let client = match state.get_client().await {
            Some(c) => c,
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

        let services: Api<Service> = if let Some(ns) = query.namespace {
            Api::namespaced(client, &ns)
        } else {
            Api::all(client)
        };

        match services.list(&ListParams::default()).await {
            Ok(svc_list) => {
                let mut result = Vec::new();

                for svc in svc_list.items {
                    let name = svc.metadata.name.unwrap_or_default();
                    let namespace = svc.metadata.namespace.unwrap_or_default();

                    let service_type = svc
                        .spec
                        .as_ref()
                        .and_then(|s| s.type_.clone())
                        .unwrap_or_else(|| "ClusterIP".to_string());

                    let cluster_ip = svc
                        .spec
                        .as_ref()
                        .and_then(|s| s.cluster_ip.clone())
                        .unwrap_or_else(|| "None".to_string());

                    let external_ip = svc
                        .status
                        .as_ref()
                        .and_then(|s| s.load_balancer.as_ref())
                        .and_then(|lb| lb.ingress.as_ref())
                        .and_then(|ing| ing.first())
                        .and_then(|i| i.ip.clone())
                        .unwrap_or_else(|| "None".to_string());

                    let ports = svc
                        .spec
                        .as_ref()
                        .and_then(|s| s.ports.as_ref())
                        .map(|ports| {
                            ports
                                .iter()
                                .map(|p| {
                                    format!(
                                        "{}:{}",
                                        p.port,
                                        p.target_port
                                            .as_ref()
                                            .map(|tp| match tp {
                                                k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(i) => i.to_string(),
                                                k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::String(s) => s.clone(),
                                            })
                                            .unwrap_or_else(|| "?".to_string())
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| "None".to_string());

                    let age = svc
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| format_age(&t.0))
                        .unwrap_or_else(|| "Unknown".to_string());

                    result.push(ServiceInfo {
                        name,
                        namespace,
                        service_type,
                        cluster_ip,
                        external_ip,
                        ports,
                        age,
                    });
                }

                Ok(Json(result))
            }
            Err(e) => {
                tracing::error!("Failed to list services: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

/// Get all nodes
pub async fn get_nodes(
    State(state): State<K8sApiState>,
) -> Result<Json<Vec<NodeInfo>>, StatusCode> {
    #[cfg(feature = "web")]
    {
        let client = match state.get_client().await {
            Some(c) => c,
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

        let nodes: Api<Node> = Api::all(client);

        match nodes.list(&ListParams::default()).await {
            Ok(node_list) => {
                let mut result = Vec::new();

                for node in node_list.items {
                    let name = node.metadata.name.unwrap_or_default();

                    let status = node
                        .status
                        .as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .and_then(|conds| conds.iter().find(|c| c.type_ == "Ready"))
                        .map(|c| c.status.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let roles = node
                        .metadata
                        .labels
                        .as_ref()
                        .map(|labels| {
                            labels
                                .iter()
                                .filter_map(|(k, _)| {
                                    if k.starts_with("node-role.kubernetes.io/") {
                                        Some(k.trim_start_matches("node-role.kubernetes.io/").to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| "none".to_string());

                    let age = node
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|t| format_age(&t.0))
                        .unwrap_or_else(|| "Unknown".to_string());

                    let version = node
                        .status
                        .as_ref()
                        .and_then(|s| s.node_info.as_ref())
                        .map(|ni| ni.kubelet_version.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let os_image = node
                        .status
                        .as_ref()
                        .and_then(|s| s.node_info.as_ref())
                        .map(|ni| ni.os_image.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let kernel_version = node
                        .status
                        .as_ref()
                        .and_then(|s| s.node_info.as_ref())
                        .map(|ni| ni.kernel_version.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let container_runtime = node
                        .status
                        .as_ref()
                        .and_then(|s| s.node_info.as_ref())
                        .map(|ni| ni.container_runtime_version.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    result.push(NodeInfo {
                        name,
                        status,
                        roles,
                        age,
                        version,
                        os_image,
                        kernel_version,
                        container_runtime,
                    });
                }

                Ok(Json(result))
            }
            Err(e) => {
                tracing::error!("Failed to list nodes: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

/// Get cluster overview
pub async fn get_cluster_overview(
    State(state): State<K8sApiState>,
) -> Result<Json<ClusterOverview>, StatusCode> {
    #[cfg(feature = "web")]
    {
        let client = match state.get_client().await {
            Some(c) => c,
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

        // Get all resources in parallel
        let namespaces_api: Api<Namespace> = Api::all(client.clone());
        let pods_api: Api<Pod> = Api::all(client.clone());
        let deployments_api: Api<Deployment> = Api::all(client.clone());
        let services_api: Api<Service> = Api::all(client.clone());
        let nodes_api: Api<Node> = Api::all(client);

        let list_params = ListParams::default();

        let (ns_result, pods_result, deploy_result, svc_result, nodes_result) = tokio::join!(
            namespaces_api.list(&list_params),
            pods_api.list(&list_params),
            deployments_api.list(&list_params),
            services_api.list(&list_params),
            nodes_api.list(&list_params),
        );

        let namespaces = ns_result.map(|l| l.items.len()).unwrap_or(0);
        let pods_list = pods_result.ok();
        let pods = pods_list.as_ref().map(|l| l.items.len()).unwrap_or(0);
        let deployments = deploy_result.map(|l| l.items.len()).unwrap_or(0);
        let services = svc_result.map(|l| l.items.len()).unwrap_or(0);
        let nodes = nodes_result.map(|l| l.items.len()).unwrap_or(0);

        // Count healthy vs unhealthy pods
        let (healthy_pods, unhealthy_pods) = pods_list
            .as_ref()
            .map(|list| {
                let healthy = list
                    .items
                    .iter()
                    .filter(|p| {
                        p.status
                            .as_ref()
                            .and_then(|s| s.phase.as_ref())
                            .map(|phase| phase == "Running" || phase == "Succeeded")
                            .unwrap_or(false)
                    })
                    .count();
                let unhealthy = list.items.len() - healthy;
                (healthy, unhealthy)
            })
            .unwrap_or((0, 0));

        Ok(Json(ClusterOverview {
            namespaces,
            pods,
            deployments,
            services,
            nodes,
            healthy_pods,
            unhealthy_pods,
        }))
    }

    #[cfg(not(feature = "web"))]
    {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

/// Format age from timestamp
#[cfg(feature = "web")]
fn format_age(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*timestamp);

    if duration.num_days() > 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m", duration.num_minutes())
    } else {
        format!("{}s", duration.num_seconds())
    }
}

/// Query parameters for manifest files
#[derive(Debug, Deserialize)]
pub struct ManifestsQuery {
    pub project_path: String,
}

/// Manifest file information
#[derive(Debug, Serialize)]
pub struct ManifestFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: String,
    pub kind: Option<String>,  // Kubernetes resource kind (Deployment, Service, etc.)
}

/// Get manifest files from project
pub async fn get_manifests(
    Query(query): Query<ManifestsQuery>,
) -> Result<Json<Vec<ManifestFile>>, StatusCode> {
    use std::path::Path;

    let project_path = Path::new(&query.project_path);
    if !project_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Check for k8s directories
    let k8s_dirs = ["k8s", "kubernetes", "deploy", "deployment", "manifests", ".kube"];
    let mut all_manifests = Vec::new();

    for dir_name in &k8s_dirs {
        let k8s_dir = project_path.join(dir_name);
        if !k8s_dir.exists() || !k8s_dir.is_dir() {
            continue;
        }

        // Recursively find .yaml and .yml files
        if let Ok(entries) = std::fs::read_dir(&k8s_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "yaml" || ext == "yml" {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                // Try to extract Kubernetes kind from file
                                let kind = extract_k8s_kind(&path);

                                all_manifests.push(ManifestFile {
                                    name: path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    path: path.to_string_lossy().to_string(),
                                    size: metadata.len(),
                                    modified: format_file_time(metadata.modified().ok()),
                                    kind,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    all_manifests.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(all_manifests))
}

/// Extract Kubernetes kind from YAML file
fn extract_k8s_kind(path: &std::path::Path) -> Option<String> {
    if let Ok(content) = std::fs::read_to_string(path) {
        // Simple regex to find "kind: ..." in YAML
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("kind:") {
                return trimmed.strip_prefix("kind:")
                    .map(|s| s.trim().to_string());
            }
        }
    }
    None
}

/// Format file modified time
fn format_file_time(time: Option<std::time::SystemTime>) -> String {
    if let Some(system_time) = time {
        if let Ok(duration) = system_time.duration_since(std::time::UNIX_EPOCH) {
            let datetime = chrono::DateTime::<chrono::Utc>::from(
                std::time::UNIX_EPOCH + duration
            );
            return datetime.format("%Y-%m-%d %H:%M").to_string();
        }
    }
    "Unknown".to_string()
}
