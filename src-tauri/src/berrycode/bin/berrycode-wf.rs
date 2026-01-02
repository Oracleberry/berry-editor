//! BerryCode Workflow Server
//!
//! A dedicated server for visual workflow building and execution.
//! Runs on port 7777, completely separate from the main editor.

use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use berrycode::{
    pipeline::{create_full_dev_pipeline, create_tdd_loop_preset},
    project_manager::ProjectManager,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tera::Tera;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing::{info, warn};
use chrono;

use berrycode::pipeline::WorkflowProgressMessage;

/// Application state
#[derive(Clone)]
struct AppState {
    tera: Arc<Tera>,
    project_manager: Arc<Mutex<ProjectManager>>,
    /// 実行中のワークフロー
    running_workflows: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// 進捗ブロードキャスター
    progress_broadcasters: Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<WorkflowProgressMessage>>>>,
    /// LLMクライアント
    llm_client: Arc<berrycode::llm::LLMClient>,
}

/// Workflow execution request
#[derive(Debug, Deserialize)]
struct ExecuteWorkflowRequest {
    /// 保存済みワークフローのID、またはプリセットID (tdd-loop, full-dev)
    workflow_id: Option<String>,
    /// カスタムワークフロー定義（workflow_idがない場合）
    nodes: Option<Vec<WorkflowNodeDef>>,
    start_node_id: Option<String>,
    /// プロジェクトパス
    project_path: String,
    /// 初期コンテキスト（オプション、最初のノードに渡される）
    initial_context: Option<String>,
}

/// Workflow execution response
#[derive(Debug, Serialize)]
struct ExecuteWorkflowResponse {
    success: bool,
    execution_id: String,
    message: String,
}

/// Workflow validation request
#[derive(Debug, Deserialize)]
struct ValidateWorkflowRequest {
    nodes: Vec<WorkflowNodeDef>,
    start_node_id: Option<String>,
}

/// Workflow node definition
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowNodeDef {
    id: String,
    name: String,
    /// アクションタイプ: design, implement, test, fix, refactor, doc, custom, http, script
    action_type: String,
    /// 成功時の遷移先ノードID
    next_on_success: Option<String>,
    /// 失敗時の遷移先ノードID
    next_on_failure: Option<String>,

    // ノード固有の設定
    /// BerryCode Agentロール（design/implement/test/fix/refactor/doc用）
    /// 指定するとそのAgentの専用プロンプトが使用される
    #[serde(default)]
    agent_role: Option<String>, // "Architect", "Programmer", "QAEngineer", "BugFixer", "Refactorer", "DocWriter"

    /// カスタムプロンプト（agent_roleがない場合、またはcustomアクション用）
    #[serde(default)]
    prompt: Option<String>,

    /// HTTPリクエストの設定（httpアクション用）
    #[serde(default)]
    http_config: Option<HttpConfig>,
    /// スクリプト実行の設定（scriptアクション用）
    #[serde(default)]
    script_config: Option<ScriptConfig>,
    /// 追加の設定（JSON形式）
    #[serde(default)]
    config: Option<serde_json::Value>,
}

/// HTTP request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpConfig {
    url: String,
    method: String, // GET, POST, PUT, DELETE
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
}

/// Script execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScriptConfig {
    command: String,
    args: Option<Vec<String>>,
    working_dir: Option<String>,
}

/// Workflow validation response
#[derive(Debug, Serialize)]
struct ValidateWorkflowResponse {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

/// Saved workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedWorkflow {
    id: String,
    name: String,
    description: Option<String>,
    /// プロジェクトパス（必須、Gitリポジトリのルート）
    project_path: String,
    nodes: Vec<WorkflowNodeDef>,
    start_node_id: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    deleted: bool,
}

/// Save workflow request
#[derive(Debug, Deserialize)]
struct SaveWorkflowRequest {
    id: Option<String>,
    name: String,
    description: Option<String>,
    /// プロジェクトパス（必須）
    project_path: String,
    nodes: Vec<WorkflowNodeDef>,
    start_node_id: Option<String>,
}

/// Save workflow response
#[derive(Debug, Serialize)]
struct SaveWorkflowResponse {
    success: bool,
    workflow_id: String,
    message: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting BerryCode Workflow Server");
    info!("Port: 7777");

    // Initialize Tera templates
    let tera = match Tera::new("templates/**/*") {
        Ok(t) => {
            #[cfg(debug_assertions)]
            info!("🔥 Debug mode: Templates will be reloaded on each request");
            t
        }
        Err(e) => {
            warn!("Failed to load templates: {}", e);
            Tera::default()
        }
    };

    // Initialize project manager
    let project_manager = Arc::new(Mutex::new(ProjectManager::new()?));

    // Initialize LLM client
    use berrycode::llm::LLMClient;
    use berrycode::models::Model;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .unwrap_or_else(|_| {
            warn!("No API key found in environment. Agent execution will fail.");
            String::new()
        });

    // モデル名を環境変数から読み込む（デフォルト: deepseek-reasoner）
    let model_name = std::env::var("BERRYCODE_MODEL")
        .unwrap_or_else(|_| "deepseek-reasoner".to_string());

    info!("Using model: {}", model_name);

    let model = Model::new(
        model_name,
        None,  // weak_model
        None,  // editor_model
        Some("diff".to_string()),  // editor_edit_format
        false,  // verbose
    )?;

    let mut llm_client = LLMClient::new(&model, api_key)?;

    // API baseを環境変数から設定
    if let Ok(api_base) = std::env::var("OPENAI_API_BASE") {
        info!("Using custom API base: {}", api_base);
        llm_client.set_api_base(api_base);
    }

    let llm_client = Arc::new(llm_client);

    let state = AppState {
        tera: Arc::new(tera),
        project_manager,
        running_workflows: Arc::new(Mutex::new(HashMap::new())),
        progress_broadcasters: Arc::new(Mutex::new(HashMap::new())),
        llm_client,
    };

    // ワークフロー保存ディレクトリを作成
    let workflows_dir = std::path::PathBuf::from("data/workflows");
    if !workflows_dir.exists() {
        std::fs::create_dir_all(&workflows_dir)?;
        info!("Created workflows directory: {:?}", workflows_dir);
    }

    // Build router
    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/editor", get(editor_handler))
        .route("/workflows", get(workflows_page_handler))
        .route("/api/projects", get(list_projects_handler))
        .route("/api/projects", axum::routing::delete(delete_project_handler))
        .route("/api/projects/add", post(add_project_handler))
        .route("/api/projects/new", post(create_project_handler))
        .route("/api/projects/clone", post(clone_repository_handler))
        .route("/api/projects/files", post(list_project_files_handler))
        .route("/api/projects/read-files", post(read_project_files_handler))
        .route("/api/workflow/execute", post(execute_workflow_handler))
        .route("/api/workflow/validate", post(validate_workflow_handler))
        .route("/api/workflows", get(list_workflows_handler))
        .route("/api/workflows", post(save_workflow_handler))
        .route("/api/workflows/:id", get(get_workflow_handler))
        .route("/api/workflows/:id", axum::routing::delete(delete_workflow_handler))
        .route("/api/chat/agent", post(chat_with_agent_handler))
        .route("/api/save-document", post(save_document_handler))
        .route("/api/workflow/generate", post(generate_workflow_handler))
        .route("/api/ws/:execution_id", get(ws_handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    // Start server
    let addr = "127.0.0.1:7777";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("🚀 BerryCode Workflow Server starting on http://{}", addr);
    info!("   Open your browser and navigate to the URL above");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Dashboard handler - main landing page
async fn dashboard_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    // Reload templates in debug mode for hot reload
    #[cfg(debug_assertions)]
    let tera = Tera::new("templates/**/*").unwrap_or_default();
    #[cfg(not(debug_assertions))]
    let tera = &state.tera;

    match tera.render("dashboard.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            warn!("Failed to render dashboard template: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        }
    }
}

/// Editor handler - workflow visual editor
async fn editor_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    // Reload templates in debug mode for hot reload
    #[cfg(debug_assertions)]
    let tera = Tera::new("templates/**/*").unwrap_or_default();
    #[cfg(not(debug_assertions))]
    let tera = &state.tera;

    // Use v2 builder with n8n-style canvas
    match tera.render("workflow_builder_v2.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            warn!("Failed to render editor template: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        }
    }
}

/// Workflows list page handler
async fn workflows_page_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    // Reload templates in debug mode for hot reload
    #[cfg(debug_assertions)]
    let tera = Tera::new("templates/**/*").unwrap_or_default();
    #[cfg(not(debug_assertions))]
    let tera = &state.tera;

    match tera.render("workflows.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            warn!("Failed to render workflows template: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        }
    }
}

/// List projects handler
async fn list_projects_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let pm = state.project_manager.lock().await;
    let projects = pm.list_projects();

    Json(serde_json::json!({
        "projects": projects
    }))
}

/// Execute workflow handler
async fn execute_workflow_handler(
    State(state): State<AppState>,
    Json(request): Json<ExecuteWorkflowRequest>,
) -> impl IntoResponse {
    // Generate execution ID
    let execution_id = format!("exec-{}", uuid::Uuid::new_v4());

    // プロジェクトパスをPathBufに変換
    let project_root = std::path::PathBuf::from(&request.project_path);

    // プロジェクトの存在確認
    if !project_root.exists() {
        return Json(ExecuteWorkflowResponse {
            success: false,
            execution_id: execution_id.clone(),
            message: format!("プロジェクトが見つかりません: {}", request.project_path),
        });
    }

    // ワークフロー定義を取得
    let (workflow_name, nodes, start_node_id) = if let Some(wf_id) = &request.workflow_id {
        // プリセットまたは保存済みワークフローをロード
        match wf_id.as_str() {
            "tdd-loop" | "full-dev" => {
                // プリセットの場合は従来のPipeline実行
                info!("Executing preset workflow '{}' on project: {}", wf_id, request.project_path);

                let pipeline = match wf_id.as_str() {
                    "tdd-loop" => create_tdd_loop_preset(),
                    "full-dev" => create_full_dev_pipeline(),
                    _ => unreachable!(),
                };

                let pipeline_name = pipeline.name.clone();
                let initial_context = request.initial_context.clone().unwrap_or_default();
                let exec_id = execution_id.clone();

                // 進捗ブロードキャスターを作成
                let (progress_tx, _) = tokio::sync::broadcast::channel::<WorkflowProgressMessage>(100);
                state.progress_broadcasters
                    .lock()
                    .await
                    .insert(exec_id.clone(), progress_tx.clone());

                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                let progress_tx_clone = progress_tx.clone();

                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        let _ = progress_tx_clone.send(msg);
                    }
                });

                // プリセットパイプライン実行
                let handle = tokio::spawn(async move {
                    info!("Starting preset pipeline execution: {} ({})", pipeline_name, exec_id);

                    match pipeline.run(
                        &project_root,
                        initial_context,
                        Some(tx),
                        Some(exec_id.clone()),
                        None, None, None, None,
                    ).await {
                        Ok(context) => {
                            info!("Pipeline completed: {} nodes, {} loops",
                                context.execution_history.len(), context.loop_count);
                        }
                        Err(e) => {
                            warn!("Pipeline failed: {} - {}", pipeline_name, e);
                        }
                    }
                });

                state.running_workflows.lock().await.insert(execution_id.clone(), handle);

                return Json(ExecuteWorkflowResponse {
                    success: true,
                    execution_id: execution_id.clone(),
                    message: format!("Preset workflow execution started: {}", execution_id),
                });
            }
            _ => {
                // 保存済みワークフローをロード
                let workflows_dir = std::path::PathBuf::from("data/workflows");
                let workflow_path = workflows_dir.join(format!("{}.json", wf_id));

                if !workflow_path.exists() {
                    return Json(ExecuteWorkflowResponse {
                        success: false,
                        execution_id: execution_id.clone(),
                        message: format!("ワークフローが見つかりません: {}", wf_id),
                    });
                }

                match std::fs::read_to_string(&workflow_path) {
                    Ok(content) => {
                        match serde_json::from_str::<SavedWorkflow>(&content) {
                            Ok(workflow) => {
                                info!("Executing saved workflow '{}' ({}) on project: {}",
                                    workflow.name, wf_id, request.project_path);
                                (workflow.name, workflow.nodes, workflow.start_node_id)
                            }
                            Err(e) => {
                                return Json(ExecuteWorkflowResponse {
                                    success: false,
                                    execution_id: execution_id.clone(),
                                    message: format!("ワークフローの解析に失敗: {}", e),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        return Json(ExecuteWorkflowResponse {
                            success: false,
                            execution_id: execution_id.clone(),
                            message: format!("ワークフローの読み込みに失敗: {}", e),
                        });
                    }
                }
            }
        }
    } else if let Some(custom_nodes) = request.nodes {
        // カスタムワークフロー
        info!("Executing custom workflow on project: {}", request.project_path);
        ("Custom Workflow".to_string(), custom_nodes, request.start_node_id)
    } else {
        return Json(ExecuteWorkflowResponse {
            success: false,
            execution_id: execution_id.clone(),
            message: "workflow_idまたはnodesのいずれかを指定してください".to_string(),
        });
    };

    // カスタムワークフロー実行
    info!("Executing custom workflow '{}' with {} nodes", workflow_name, nodes.len());

    // 開始ノードの確認
    let start_id = match start_node_id {
        Some(id) => id,
        None => {
            return Json(ExecuteWorkflowResponse {
                success: false,
                execution_id: execution_id.clone(),
                message: "開始ノードが指定されていません".to_string(),
            });
        }
    };

    let exec_id = execution_id.clone();
    let initial_ctx = request.initial_context.clone().unwrap_or_default();

    // 進捗ブロードキャスターを作成
    let (progress_tx, _) = tokio::sync::broadcast::channel::<WorkflowProgressMessage>(100);
    state.progress_broadcasters
        .lock()
        .await
        .insert(exec_id.clone(), progress_tx.clone());

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let progress_tx_clone = progress_tx.clone();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = progress_tx_clone.send(msg);
        }
    });

    // カスタムワークフローをバックグラウンドで実行
    let workflow_name_clone = workflow_name.clone();
    let llm_client_clone = state.llm_client.clone();
    let handle = tokio::spawn(async move {
        info!("Starting custom workflow execution: {} ({})", workflow_name_clone, exec_id);

        match execute_custom_workflow(
            &project_root,
            &nodes,
            &start_id,
            &initial_ctx,
            tx,
            llm_client_clone,
        ).await {
            Ok(result) => {
                info!("Custom workflow completed: {} ({})", workflow_name_clone, exec_id);
                info!("Result: {}", result);
            }
            Err(e) => {
                warn!("Custom workflow failed: {} ({}): {}", workflow_name_clone, exec_id, e);
            }
        }
    });

    state.running_workflows.lock().await.insert(execution_id.clone(), handle);

    Json(ExecuteWorkflowResponse {
        success: true,
        execution_id: execution_id.clone(),
        message: format!("Custom workflow execution started: {}", execution_id),
    })
}

/// カスタムワークフローを実行
async fn execute_custom_workflow(
    project_root: &std::path::PathBuf,
    nodes: &[WorkflowNodeDef],
    start_node_id: &str,
    initial_context: &str,
    progress_tx: tokio::sync::mpsc::UnboundedSender<WorkflowProgressMessage>,
    llm_client: Arc<berrycode::llm::LLMClient>,
) -> anyhow::Result<String> {
    use std::collections::HashMap;

    // ノードマップを作成
    let node_map: HashMap<&str, &WorkflowNodeDef> = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();

    // 現在のノードIDとコンテキスト
    let mut current_node_id = start_node_id;
    let mut context = initial_context.to_string();
    let mut visited_count = 0;
    const MAX_ITERATIONS: usize = 100; // 無限ループ防止

    loop {
        if visited_count >= MAX_ITERATIONS {
            return Err(anyhow::anyhow!("最大反復回数に達しました（無限ループの可能性）"));
        }
        visited_count += 1;

        // 現在のノードを取得
        let node = match node_map.get(current_node_id) {
            Some(n) => n,
            None => {
                return Err(anyhow::anyhow!("ノードが見つかりません: {}", current_node_id));
            }
        };

        info!("Executing node: {} ({})", node.name, node.action_type);

        // 進捗送信
        let _ = progress_tx.send(WorkflowProgressMessage {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            status: "running".to_string(),
            message: format!("実行中: {}", node.name),
            loop_count: visited_count,
        });

        // ノードのアクションを実行
        let (success, output) = execute_node_action(node, project_root, &context, llm_client.clone()).await?;

        // コンテキストを更新
        context = output.clone();

        // 次のノードを決定
        current_node_id = if success {
            match &node.next_on_success {
                Some(next_id) => next_id.as_str(),
                None => {
                    // 終端ノード
                    info!("Workflow completed successfully at node: {}", node.name);
                    return Ok(output);
                }
            }
        } else {
            match &node.next_on_failure {
                Some(next_id) => next_id.as_str(),
                None => {
                    // 失敗終端
                    return Err(anyhow::anyhow!("Workflow failed at node: {}", node.name));
                }
            }
        };
    }
}

/// docs/配下の設計書を読み込む
fn load_design_documents(project_root: &std::path::PathBuf) -> String {
    let docs_dir = project_root.join("docs");

    if !docs_dir.exists() {
        info!("docs/ directory not found in project");
        return String::new();
    }

    let mut design_docs = String::new();

    match std::fs::read_dir(&docs_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();

                // .mdファイルのみ読み込む（README.mdは除外）
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        if filename == "README.md" {
                            continue; // README.mdはスキップ
                        }

                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                info!("Loaded design document: {:?}", path);
                                design_docs.push_str(&format!("\n\n## 設計書: {}\n\n", filename));
                                design_docs.push_str(&content);
                            }
                            Err(e) => {
                                warn!("Failed to read design document {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to read docs directory: {}", e);
        }
    }

    if !design_docs.is_empty() {
        info!("Loaded {} bytes of design documents", design_docs.len());
    }

    design_docs
}

/// 個別のノードアクションを実行
async fn execute_node_action(
    node: &WorkflowNodeDef,
    project_root: &std::path::PathBuf,
    context: &str,
    llm_client: Arc<berrycode::llm::LLMClient>,
) -> anyhow::Result<(bool, String)> {
    match node.action_type.as_str() {
        "design" | "implement" | "test" | "fix" | "refactor" | "doc" => {
            // BerryCode Agentを使用
            use berrycode::agents::{AgentRole, AgentContext, AgentConfig, create_agent};
            use std::collections::HashMap;

            // AgentRoleを決定
            let agent_role = if let Some(role_str) = &node.agent_role {
                match role_str.as_str() {
                    "Architect" => AgentRole::Architect,
                    "Programmer" => AgentRole::Programmer,
                    "QAEngineer" => AgentRole::QAEngineer,
                    "BugFixer" => AgentRole::BugFixer,
                    "Refactorer" => AgentRole::Refactorer,
                    "DocWriter" => AgentRole::DocWriter,
                    _ => {
                        match node.action_type.as_str() {
                            "design" => AgentRole::Architect,
                            "implement" => AgentRole::Programmer,
                            "test" => AgentRole::QAEngineer,
                            "fix" => AgentRole::BugFixer,
                            "refactor" => AgentRole::Refactorer,
                            "doc" => AgentRole::DocWriter,
                            _ => AgentRole::Programmer,
                        }
                    }
                }
            } else {
                match node.action_type.as_str() {
                    "design" => AgentRole::Architect,
                    "implement" => AgentRole::Programmer,
                    "test" => AgentRole::QAEngineer,
                    "fix" => AgentRole::BugFixer,
                    "refactor" => AgentRole::Refactorer,
                    "doc" => AgentRole::DocWriter,
                    _ => AgentRole::Programmer,
                }
            };

            let agent = create_agent(agent_role);

            info!(
                "Executing {} action for node: {} using Agent: {}",
                node.action_type, node.name, agent.name()
            );

            // タスクを決定
            let task = node.prompt.as_deref().unwrap_or(context);

            // 実装ノードの場合、docs/配下の設計書を自動読み込み
            let design_docs = if node.action_type == "implement" {
                load_design_documents(project_root)
            } else {
                String::new()
            };

            // AgentContextを作成
            let mut inputs = HashMap::new();
            inputs.insert("task".to_string(), task.to_string());
            inputs.insert("context".to_string(), context.to_string());
            inputs.insert("requirement".to_string(), task.to_string());

            // 設計書がある場合は追加
            if !design_docs.is_empty() {
                inputs.insert("design_documents".to_string(), design_docs.clone());
                info!("Added design documents to agent context");

                // タスクに設計書参照の指示を追加
                let enhanced_task = format!(
                    "{}\n\n# 設計書\n\n以下の設計書に基づいて実装してください。まだ実装されていない部分があれば優先的に実装してください。\n\n{}",
                    task,
                    design_docs
                );
                inputs.insert("task".to_string(), enhanced_task);
            }

            let agent_context = AgentContext {
                project_root: project_root.clone(),
                inputs,
                config: AgentConfig::default(),
                llm_client,
                repo_map: None, // TODO: RepoMapを渡す場合は構築が必要
            };

            // 実際のAgent実行
            match agent.execute(&agent_context).await {
                Ok(output) => {
                    info!("Agent {} completed successfully", agent.name());

                    // 結果をフォーマット
                    let result_text = if !output.files.is_empty() {
                        let mut result = format!("Agent: {}\n\nFiles modified:\n", agent.name());
                        for (path, _content) in output.files.iter() {
                            result.push_str(&format!("  - {}\n", path.display()));
                        }
                        result.push_str(&format!("\nMessage: {}\n", output.message));
                        result
                    } else {
                        format!("Agent: {}\n\nMessage: {}\n", agent.name(), output.message)
                    };

                    Ok((output.success, result_text))
                }
                Err(e) => {
                    warn!("Agent {} failed: {}", agent.name(), e);
                    Ok((false, format!("Agent execution failed: {}", e)))
                }
            }
        }
        "custom" => {
            // カスタムプロンプト実行
            let prompt = node.prompt.as_deref().unwrap_or("デフォルトプロンプト");
            info!("Executing custom action with prompt: {}", prompt);
            Ok((true, format!("カスタムアクション結果: {}", prompt)))
        }
        "http" => {
            // HTTPリクエスト実行
            if let Some(http_config) = &node.http_config {
                info!("Executing HTTP {} to {}", http_config.method, http_config.url);

                // 実際のHTTPリクエスト（簡易実装）
                let client = reqwest::Client::new();
                let response = match http_config.method.to_uppercase().as_str() {
                    "GET" => client.get(&http_config.url).send().await?,
                    "POST" => {
                        let mut req = client.post(&http_config.url);
                        if let Some(body) = &http_config.body {
                            req = req.body(body.clone());
                        }
                        req.send().await?
                    }
                    _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", http_config.method)),
                };

                let status = response.status();
                let body = response.text().await?;

                Ok((status.is_success(), body))
            } else {
                Err(anyhow::anyhow!("HTTP設定がありません"))
            }
        }
        "script" => {
            // スクリプト実行
            if let Some(script_config) = &node.script_config {
                info!("Executing script: {}", script_config.command);

                let mut cmd = tokio::process::Command::new(&script_config.command);

                if let Some(args) = &script_config.args {
                    cmd.args(args);
                }

                if let Some(working_dir) = &script_config.working_dir {
                    cmd.current_dir(working_dir);
                } else {
                    cmd.current_dir(project_root);
                }

                let output = cmd.output().await?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let result = if stdout.is_empty() { stderr } else { stdout };

                Ok((output.status.success(), result))
            } else {
                Err(anyhow::anyhow!("スクリプト設定がありません"))
            }
        }
        _ => {
            Err(anyhow::anyhow!("Unknown action type: {}", node.action_type))
        }
    }
}

/// Validate workflow handler
async fn validate_workflow_handler(
    Json(request): Json<ValidateWorkflowRequest>,
) -> impl IntoResponse {
    info!("Validating workflow with {} nodes", request.nodes.len());

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check if workflow has nodes
    if request.nodes.is_empty() {
        errors.push("ワークフローにノードがありません".to_string());
        return Json(ValidateWorkflowResponse {
            valid: false,
            errors,
            warnings,
        });
    }

    // Check if start node is specified
    if request.start_node_id.is_none() {
        errors.push("開始ノードが指定されていません".to_string());
    }

    // Build node map
    let mut node_map: HashMap<String, &WorkflowNodeDef> = HashMap::new();
    for node in &request.nodes {
        if node_map.contains_key(&node.id) {
            errors.push(format!("重複するノードID: {}", node.id));
        }
        node_map.insert(node.id.clone(), node);
    }

    // Check if start node exists
    if let Some(start_id) = &request.start_node_id {
        if !node_map.contains_key(start_id) {
            errors.push(format!("開始ノード '{}' が見つかりません", start_id));
        }
    }

    // Check connections validity
    for node in &request.nodes {
        if let Some(next_id) = &node.next_on_success {
            if !node_map.contains_key(next_id) {
                errors.push(format!(
                    "ノード '{}' の成功時遷移先 '{}' が見つかりません",
                    node.name, next_id
                ));
            }
        }
        if let Some(next_id) = &node.next_on_failure {
            if !node_map.contains_key(next_id) {
                errors.push(format!(
                    "ノード '{}' の失敗時遷移先 '{}' が見つかりません",
                    node.name, next_id
                ));
            }
        }
    }

    // Check for cycles (using DFS)
    if let Some(start_id) = &request.start_node_id {
        if node_map.contains_key(start_id) {
            let has_cycle = detect_cycle(start_id, &node_map);
            if has_cycle {
                errors.push("ワークフローに無限ループが検出されました".to_string());
            }
        }
    }

    // Check for unreachable nodes
    if let Some(start_id) = &request.start_node_id {
        if node_map.contains_key(start_id) {
            let reachable = find_reachable_nodes(start_id, &node_map);
            for node in &request.nodes {
                if !reachable.contains(&node.id) {
                    warnings.push(format!(
                        "ノード '{}' は開始ノードから到達できません",
                        node.name
                    ));
                }
            }
        }
    }

    // Check for nodes without any outgoing connections (potential dead ends)
    for node in &request.nodes {
        if node.next_on_success.is_none() && node.next_on_failure.is_none() {
            warnings.push(format!(
                "ノード '{}' には遷移先がありません（終端ノード）",
                node.name
            ));
        }
    }

    let valid = errors.is_empty();
    info!(
        "Validation complete: valid={}, errors={}, warnings={}",
        valid,
        errors.len(),
        warnings.len()
    );

    Json(ValidateWorkflowResponse {
        valid,
        errors,
        warnings,
    })
}

/// Detect cycles in workflow graph using DFS
fn detect_cycle(
    start_id: &str,
    node_map: &HashMap<String, &WorkflowNodeDef>,
) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    dfs_cycle_detection(start_id, node_map, &mut visited, &mut rec_stack)
}

fn dfs_cycle_detection(
    node_id: &str,
    node_map: &HashMap<String, &WorkflowNodeDef>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
) -> bool {
    if rec_stack.contains(node_id) {
        return true; // Cycle detected
    }
    if visited.contains(node_id) {
        return false; // Already visited, no cycle from here
    }

    visited.insert(node_id.to_string());
    rec_stack.insert(node_id.to_string());

    if let Some(node) = node_map.get(node_id) {
        // Check success path
        if let Some(next_id) = &node.next_on_success {
            if dfs_cycle_detection(next_id, node_map, visited, rec_stack) {
                return true;
            }
        }
        // Check failure path
        if let Some(next_id) = &node.next_on_failure {
            if dfs_cycle_detection(next_id, node_map, visited, rec_stack) {
                return true;
            }
        }
    }

    rec_stack.remove(node_id);
    false
}

/// Find all reachable nodes from start node
fn find_reachable_nodes(
    start_id: &str,
    node_map: &HashMap<String, &WorkflowNodeDef>,
) -> HashSet<String> {
    let mut reachable = HashSet::new();
    dfs_reachability(start_id, node_map, &mut reachable);
    reachable
}

fn dfs_reachability(
    node_id: &str,
    node_map: &HashMap<String, &WorkflowNodeDef>,
    reachable: &mut HashSet<String>,
) {
    if reachable.contains(node_id) {
        return; // Already visited
    }

    reachable.insert(node_id.to_string());

    if let Some(node) = node_map.get(node_id) {
        if let Some(next_id) = &node.next_on_success {
            dfs_reachability(next_id, node_map, reachable);
        }
        if let Some(next_id) = &node.next_on_failure {
            dfs_reachability(next_id, node_map, reachable);
        }
    }
}

/// Query parameters for listing workflows
#[derive(Debug, Deserialize)]
struct ListWorkflowsQuery {
    project_path: Option<String>,
}

/// List saved workflows handler
async fn list_workflows_handler(
    axum::extract::Query(query): axum::extract::Query<ListWorkflowsQuery>,
) -> impl IntoResponse {
    let workflows_dir = std::path::PathBuf::from("data/workflows");

    let mut workflows = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&workflows_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(workflow) = serde_json::from_str::<SavedWorkflow>(&content) {
                        // Skip deleted workflows
                        if workflow.deleted {
                            continue;
                        }

                        // Filter by project_path if specified
                        if let Some(ref filter_path) = query.project_path {
                            if workflow.project_path == *filter_path {
                                workflows.push(workflow);
                            }
                        } else {
                            workflows.push(workflow);
                        }
                    }
                }
            }
        }
    }

    // Sort by updated_at descending
    workflows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Json(serde_json::json!({
        "workflows": workflows
    }))
}

/// Save workflow handler
async fn save_workflow_handler(
    Json(request): Json<SaveWorkflowRequest>,
) -> impl IntoResponse {
    info!("Saving workflow: {}", request.name);

    let workflow_id = request.id.unwrap_or_else(|| format!("wf-{}", uuid::Uuid::new_v4()));
    let now = chrono::Utc::now().to_rfc3339();

    // Check if workflow exists to preserve created_at
    let workflows_dir = std::path::PathBuf::from("data/workflows");
    let workflow_path = workflows_dir.join(format!("{}.json", workflow_id));

    let created_at = if workflow_path.exists() {
        // Load existing workflow to preserve created_at
        match std::fs::read_to_string(&workflow_path) {
            Ok(content) => {
                match serde_json::from_str::<SavedWorkflow>(&content) {
                    Ok(existing) => existing.created_at,
                    Err(_) => now.clone(),
                }
            }
            Err(_) => now.clone(),
        }
    } else {
        now.clone()
    };

    let workflow = SavedWorkflow {
        id: workflow_id.clone(),
        name: request.name,
        description: request.description,
        project_path: request.project_path,
        nodes: request.nodes,
        start_node_id: request.start_node_id,
        created_at,
        updated_at: now,
        deleted: false,
    };

    match serde_json::to_string_pretty(&workflow) {
        Ok(json) => {
            match std::fs::write(&workflow_path, json) {
                Ok(_) => {
                    info!("Workflow saved successfully: {}", workflow_id);
                    Json(SaveWorkflowResponse {
                        success: true,
                        workflow_id: workflow_id.clone(),
                        message: format!("ワークフローを保存しました: {}", workflow_id),
                    })
                }
                Err(e) => {
                    warn!("Failed to save workflow: {}", e);
                    Json(SaveWorkflowResponse {
                        success: false,
                        workflow_id: workflow_id.clone(),
                        message: format!("保存に失敗しました: {}", e),
                    })
                }
            }
        }
        Err(e) => {
            warn!("Failed to serialize workflow: {}", e);
            Json(SaveWorkflowResponse {
                success: false,
                workflow_id: workflow_id.clone(),
                message: format!("シリアライズに失敗しました: {}", e),
            })
        }
    }
}

/// Get workflow by ID handler
async fn get_workflow_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let workflows_dir = std::path::PathBuf::from("data/workflows");
    let workflow_path = workflows_dir.join(format!("{}.json", id));

    if !workflow_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Workflow not found"
            }))
        ).into_response();
    }

    match std::fs::read_to_string(&workflow_path) {
        Ok(content) => {
            match serde_json::from_str::<SavedWorkflow>(&content) {
                Ok(workflow) => Json(workflow).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Failed to parse workflow: {}", e)
                    }))
                ).into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to read workflow: {}", e)
            }))
        ).into_response(),
    }
}

/// Delete workflow handler (logical delete)
async fn delete_workflow_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    info!("Deleting workflow (logical): {}", id);

    let workflows_dir = std::path::PathBuf::from("data/workflows");
    let workflow_path = workflows_dir.join(format!("{}.json", id));

    if !workflow_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Workflow not found"
            }))
        ).into_response();
    }

    // Read existing workflow
    match std::fs::read_to_string(&workflow_path) {
        Ok(content) => {
            match serde_json::from_str::<SavedWorkflow>(&content) {
                Ok(mut workflow) => {
                    // Set deleted flag
                    workflow.deleted = true;
                    workflow.updated_at = chrono::Utc::now().to_rfc3339();

                    // Save updated workflow
                    match serde_json::to_string_pretty(&workflow) {
                        Ok(json) => {
                            match std::fs::write(&workflow_path, json) {
                                Ok(_) => {
                                    info!("Workflow logically deleted: {}", id);
                                    Json(serde_json::json!({
                                        "success": true,
                                        "message": "ワークフローを削除しました"
                                    })).into_response()
                                }
                                Err(e) => {
                                    warn!("Failed to save deleted workflow: {}", e);
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(serde_json::json!({
                                            "error": format!("削除に失敗しました: {}", e)
                                        }))
                                    ).into_response()
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to serialize workflow: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "error": format!("削除に失敗しました: {}", e)
                                }))
                            ).into_response()
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse workflow: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("ワークフローの読み込みに失敗しました: {}", e)
                        }))
                    ).into_response()
                }
            }
        }
        Err(e) => {
            warn!("Failed to read workflow file: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("ワークフローの読み込みに失敗しました: {}", e)
                }))
            ).into_response()
        }
    }
}

/// WebSocket handler for real-time progress updates
async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::Path(execution_id): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, execution_id, state))
}

async fn handle_socket(mut socket: WebSocket, execution_id: String, state: AppState) {
    info!("WebSocket connected for execution: {}", execution_id);

    // 進捗ブロードキャスターを取得
    let progress_rx = {
        let broadcasters = state.progress_broadcasters.lock().await;
        match broadcasters.get(&execution_id) {
            Some(tx) => tx.subscribe(),
            None => {
                warn!("No progress broadcaster found for execution: {}", execution_id);
                let _ = socket.send(Message::Text(
                    serde_json::json!({
                        "error": "Execution not found"
                    }).to_string()
                )).await;
                return;
            }
        }
    };

    // 進捗メッセージをWebSocketで転送
    let mut progress_rx = progress_rx;
    while let Ok(msg) = progress_rx.recv().await {
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to serialize progress message: {}", e);
                continue;
            }
        };

        if socket.send(Message::Text(json)).await.is_err() {
            info!("WebSocket client disconnected");
            break;
        }
    }

    info!("WebSocket handler completed for execution: {}", execution_id);
}

/// Generate workflow request
#[derive(Debug, Deserialize)]
struct GenerateWorkflowRequest {
    project_path: String,
}

/// Generate workflow response
#[derive(Debug, Serialize)]
struct GenerateWorkflowResponse {
    success: bool,
    workflow: GeneratedWorkflow,
    description: String,
    message: Option<String>,
}

/// Generated workflow structure
#[derive(Debug, Serialize)]
struct GeneratedWorkflow {
    nodes: Vec<GeneratedNode>,
    connections: Vec<GeneratedConnection>,
}

/// Generated node
#[derive(Debug, Serialize)]
struct GeneratedNode {
    id: String,
    type_: String,
    #[serde(rename = "type")]
    type_field: String,
    name: String,
    x: i32,
    y: i32,
    config: serde_json::Value,
}

/// Generated connection
#[derive(Debug, Serialize)]
struct GeneratedConnection {
    #[serde(rename = "fromNodeId")]
    from_node_id: String,
    #[serde(rename = "toNodeId")]
    to_node_id: String,
    #[serde(rename = "fromPort")]
    from_port: String,
    #[serde(rename = "toPort")]
    to_port: String,
}

/// Generate workflow from project analysis
async fn generate_workflow_handler(
    State(state): State<AppState>,
    Json(request): Json<GenerateWorkflowRequest>,
) -> impl IntoResponse {
    info!("Generating workflow for project: {}", request.project_path);

    let project_path = std::path::PathBuf::from(&request.project_path);

    // プロジェクトの存在確認
    if !project_path.exists() {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("プロジェクトが見つかりません: {}", request.project_path)
        })).into_response();
    }

    // .gitディレクトリの存在確認
    let git_dir = project_path.join(".git");
    if !git_dir.exists() {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("このプロジェクトはGitリポジトリではありません: {}\n.gitディレクトリが見つかりません。", request.project_path)
        })).into_response();
    }

    // プロジェクト解析
    let analysis = analyze_project(&project_path).await;

    // AIにワークフロー提案を依頼
    use berrycode::llm::Message;

    let system_prompt = r#"あなたはソフトウェア開発ワークフローの専門家です。
プロジェクトの情報を分析して、最適な開発ワークフローを提案してください。

ワークフローは以下のノードタイプから構成されます:
- design: 設計・アーキテクチャ検討
- implement: コード実装
- test: テスト実行
- fix: バグ修正
- refactor: リファクタリング
- doc: ドキュメント作成

JSON形式で、以下のような構造でワークフローを提案してください:

{
  "description": "ワークフローの説明",
  "nodes": [
    {
      "id": "node-1",
      "type": "design",
      "name": "システム設計",
      "prompt": "オプションの具体的な指示"
    },
    {
      "id": "node-2",
      "type": "implement",
      "name": "実装",
      "prompt": null
    }
  ],
  "connections": [
    {
      "from": "node-1",
      "to": "node-2",
      "on": "success"
    }
  ]
}

JSONのみを出力してください（説明文などは不要です）。"#;

    let user_prompt = format!(
        r#"以下のプロジェクト情報を分析して、最適なワークフローを提案してください:

プロジェクトパス: {}
言語: {}
ファイル数: {}
主要ファイル: {}
README概要: {}

このプロジェクトに適したワークフローをJSON形式で提案してください。"#,
        request.project_path,
        analysis.language,
        analysis.file_count,
        analysis.main_files.join(", "),
        analysis.readme_summary
    );

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: "user".to_string(),
            content: Some(user_prompt),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    match state.llm_client.chat(messages).await {
        Ok((response, _, _)) => {
            info!("Workflow proposal received: {} chars", response.len());

            // JSONをパース
            let json_str = extract_json(&response);

            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(workflow_json) => {
                    // ノードを生成
                    let empty_vec = vec![];
                    let nodes_json = workflow_json["nodes"].as_array().unwrap_or(&empty_vec);
                    let mut nodes = Vec::new();

                    for (i, node_json) in nodes_json.iter().enumerate() {
                        let node_type = node_json["type"].as_str().unwrap_or("implement");
                        let node_id = node_json["id"]
                            .as_str()
                            .unwrap_or(&format!("node-{}", i + 1))
                            .to_string();

                        nodes.push(GeneratedNode {
                            id: node_id.clone(),
                            type_: node_type.to_string(),
                            type_field: node_type.to_string(),
                            name: node_json["name"]
                                .as_str()
                                .unwrap_or("Untitled")
                                .to_string(),
                            x: 100 + (i as i32 % 3) * 250,
                            y: 100 + (i as i32 / 3) * 150,
                            config: serde_json::json!({
                                "prompt": node_json.get("prompt")
                            }),
                        });
                    }

                    // コネクションを生成
                    let empty_connections = vec![];
                    let connections_json = workflow_json["connections"]
                        .as_array()
                        .unwrap_or(&empty_connections);
                    let mut connections = Vec::new();

                    for conn_json in connections_json {
                        let from_id = conn_json["from"].as_str().unwrap_or("").to_string();
                        let to_id = conn_json["to"].as_str().unwrap_or("").to_string();
                        let on = conn_json["on"].as_str().unwrap_or("success");

                        connections.push(GeneratedConnection {
                            from_node_id: from_id,
                            to_node_id: to_id,
                            from_port: on.to_string(),
                            to_port: "input".to_string(),
                        });
                    }

                    let description = workflow_json["description"]
                        .as_str()
                        .unwrap_or("AI生成ワークフロー")
                        .to_string();

                    Json(GenerateWorkflowResponse {
                        success: true,
                        workflow: GeneratedWorkflow { nodes, connections },
                        description,
                        message: None,
                    })
                    .into_response()
                }
                Err(e) => {
                    warn!("Failed to parse workflow JSON: {}", e);
                    warn!("Response: {}", response);

                    Json(serde_json::json!({
                        "success": false,
                        "message": format!("ワークフロー解析に失敗しました: {}", e)
                    }))
                    .into_response()
                }
            }
        }
        Err(e) => {
            warn!("LLM error: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("ワークフロー生成に失敗しました: {}", e)
            }))
            .into_response()
        }
    }
}

/// Project analysis result
struct ProjectAnalysis {
    language: String,
    file_count: usize,
    main_files: Vec<String>,
    readme_summary: String,
}

/// Analyze project to understand its structure
async fn analyze_project(project_path: &std::path::PathBuf) -> ProjectAnalysis {
    use std::fs;

    let mut language = "Unknown".to_string();
    let mut file_count = 0;
    let mut main_files = Vec::new();
    let mut readme_summary = "No README found".to_string();

    // 言語検出
    if project_path.join("Cargo.toml").exists() {
        language = "Rust".to_string();
        main_files.push("Cargo.toml".to_string());
        if project_path.join("src/main.rs").exists() {
            main_files.push("src/main.rs".to_string());
        }
        if project_path.join("src/lib.rs").exists() {
            main_files.push("src/lib.rs".to_string());
        }
    } else if project_path.join("package.json").exists() {
        language = "JavaScript/TypeScript".to_string();
        main_files.push("package.json".to_string());
    } else if project_path.join("requirements.txt").exists()
        || project_path.join("setup.py").exists()
    {
        language = "Python".to_string();
        if project_path.join("requirements.txt").exists() {
            main_files.push("requirements.txt".to_string());
        }
    } else if project_path.join("go.mod").exists() {
        language = "Go".to_string();
        main_files.push("go.mod".to_string());
    }

    // ファイル数をカウント
    if let Ok(entries) = fs::read_dir(project_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    file_count += 1;
                }
            }
        }
    }

    // README読み込み
    let readme_paths = ["README.md", "README.txt", "README"];
    for readme_name in &readme_paths {
        let readme_path = project_path.join(readme_name);
        if readme_path.exists() {
            if let Ok(content) = fs::read_to_string(&readme_path) {
                // 最初の500文字を概要として使用
                readme_summary = content.chars().take(500).collect();
                readme_summary = readme_summary
                    .lines()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join("\n");
                break;
            }
        }
    }

    ProjectAnalysis {
        language,
        file_count,
        main_files,
        readme_summary,
    }
}

/// Extract JSON from LLM response (remove markdown code blocks etc.)
fn extract_json(text: &str) -> String {
    // JSONブロックを抽出（```json ... ``` または { ... }）
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start..].find("```") {
            let json_content = &text[start + 7..start + end];
            return json_content.trim().to_string();
        }
    }

    // { で始まる部分を探す
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }

    text.trim().to_string()
}

/// Chat message history
#[derive(Debug, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Chat with agent request
#[derive(Debug, Deserialize)]
struct ChatWithAgentRequest {
    agent_role: String,
    message: String,
    #[serde(default)]
    history: Vec<ChatMessage>,
}

/// Chat with agent response
#[derive(Debug, Serialize)]
struct ChatWithAgentResponse {
    response: String,
}

/// Chat with agent handler - interactive design/implementation chat
async fn chat_with_agent_handler(
    State(state): State<AppState>,
    Json(request): Json<ChatWithAgentRequest>,
) -> impl IntoResponse {
    info!("Chat with agent: {} - {}", request.agent_role, request.message);

    // AgentRoleを決定
    use berrycode::agents::{AgentRole, create_agent};
    use berrycode::llm::Message;

    let agent_role = match request.agent_role.as_str() {
        "Architect" => AgentRole::Architect,
        "Programmer" => AgentRole::Programmer,
        "QAEngineer" => AgentRole::QAEngineer,
        "BugFixer" => AgentRole::BugFixer,
        "Refactorer" => AgentRole::Refactorer,
        "DocWriter" => AgentRole::DocWriter,
        _ => AgentRole::Architect, // デフォルト
    };

    let agent = create_agent(agent_role);

    // LLMに渡すメッセージを構築
    let mut messages = Vec::new();

    // システムメッセージ
    let system_content = format!(
        "あなたは{}です。\n\n{}",
        agent.name(),
        agent.system_prompt()
    );
    messages.push(Message {
        role: "system".to_string(),
        content: Some(system_content),
        tool_calls: None,
        tool_call_id: None,
    });

    // 会話履歴
    for msg in &request.history {
        messages.push(Message {
            role: msg.role.clone(),
            content: Some(msg.content.clone()),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // 現在のユーザーメッセージ
    messages.push(Message {
        role: "user".to_string(),
        content: Some(request.message.clone()),
        tool_calls: None,
        tool_call_id: None,
    });

    // LLMで回答を生成
    match state.llm_client.chat(messages).await {
        Ok((response, _input_tokens, _output_tokens)) => {
            info!("Agent response generated: {} chars", response.len());
            Json(ChatWithAgentResponse {
                response: response.trim().to_string(),
            }).into_response()
        }
        Err(e) => {
            warn!("LLM error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("LLM処理に失敗しました: {}", e)
                }))
            ).into_response()
        }
    }
}

/// Save document request
#[derive(Debug, Deserialize)]
struct SaveDocumentRequest {
    filename: String,
    content: String,
    agent_role: Option<String>,
    #[serde(default)]
    append: bool,
}

/// Save document response
#[derive(Debug, Serialize)]
struct SaveDocumentResponse {
    success: bool,
    filepath: String,
    message: String,
}

/// Save document handler - saves chat results as markdown files
async fn save_document_handler(
    Json(request): Json<SaveDocumentRequest>,
) -> impl IntoResponse {
    info!("Saving document: {}", request.filename);

    // ドキュメント保存ディレクトリを作成
    let docs_dir = std::path::PathBuf::from("docs");
    if !docs_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&docs_dir) {
            warn!("Failed to create docs directory: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("ディレクトリの作成に失敗: {}", e)
                }))
            ).into_response();
        }
    }

    // ファイルパスを作成
    let filepath = docs_dir.join(&request.filename);

    // メタデータを含むコンテンツを作成
    let mut full_content = String::new();

    if request.append && filepath.exists() {
        // 追記モード: 既存ファイルを読み込んで追記
        match std::fs::read_to_string(&filepath) {
            Ok(existing_content) => {
                full_content.push_str(&existing_content);
                full_content.push_str(&format!("\n\n---\n\n## 追記 ({})\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M")));
                full_content.push_str(&request.content);
                info!("Appending to existing document: {:?}", filepath);
            }
            Err(e) => {
                warn!("Failed to read existing file for append: {}", e);
                // 読み込み失敗時は新規作成
                full_content.push_str(&format!("---\n"));
                full_content.push_str(&format!("generated_at: {}\n", chrono::Utc::now().to_rfc3339()));
                if let Some(agent_role) = &request.agent_role {
                    full_content.push_str(&format!("agent_role: {}\n", agent_role));
                }
                full_content.push_str(&format!("---\n\n"));
                full_content.push_str(&request.content);
            }
        }
    } else {
        // 新規作成または上書きモード
        full_content.push_str(&format!("---\n"));
        full_content.push_str(&format!("generated_at: {}\n", chrono::Utc::now().to_rfc3339()));
        if let Some(agent_role) = &request.agent_role {
            full_content.push_str(&format!("agent_role: {}\n", agent_role));
        }
        full_content.push_str(&format!("---\n\n"));
        full_content.push_str(&request.content);
    }

    // ファイルに保存
    match std::fs::write(&filepath, full_content) {
        Ok(_) => {
            info!("Document saved successfully: {:?}", filepath);

            // README.mdを更新
            if let Err(e) = update_docs_readme(&docs_dir) {
                warn!("Failed to update README.md: {}", e);
            }

            Json(SaveDocumentResponse {
                success: true,
                filepath: filepath.to_string_lossy().to_string(),
                message: "設計書を保存しました".to_string(),
            }).into_response()
        }
        Err(e) => {
            warn!("Failed to save document: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("ファイルの保存に失敗: {}", e)
                }))
            ).into_response()
        }
    }
}

/// Update docs/README.md with list of all documents
fn update_docs_readme(docs_dir: &std::path::PathBuf) -> anyhow::Result<()> {
    use std::fs;

    let mut readme_content = String::new();

    // ヘッダー
    readme_content.push_str("# BerryCode ドキュメント\n\n");
    readme_content.push_str("このディレクトリには、BerryCode AIチャットで生成された設計書が保存されています。\n\n");
    readme_content.push_str(&format!("最終更新: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    // ドキュメント一覧を収集
    let mut documents: Vec<(String, String, String)> = Vec::new(); // (filename, generated_at, agent_role)

    if let Ok(entries) = fs::read_dir(docs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md")
                && path.file_name().and_then(|s| s.to_str()) != Some("README.md")
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    // メタデータを抽出
                    let mut generated_at = "不明".to_string();
                    let mut agent_role = "不明".to_string();

                    if content.starts_with("---\n") {
                        if let Some(end_idx) = content[4..].find("---\n") {
                            let metadata = &content[4..4 + end_idx];
                            for line in metadata.lines() {
                                if let Some(value) = line.strip_prefix("generated_at: ") {
                                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
                                        generated_at = dt.format("%Y-%m-%d %H:%M").to_string();
                                    }
                                } else if let Some(value) = line.strip_prefix("agent_role: ") {
                                    agent_role = value.to_string();
                                }
                            }
                        }
                    }

                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        documents.push((filename.to_string(), generated_at, agent_role));
                    }
                }
            }
        }
    }

    // 生成日時でソート（新しい順）
    documents.sort_by(|a, b| b.1.cmp(&a.1));

    // ドキュメント一覧を作成
    readme_content.push_str("## 📚 ドキュメント一覧\n\n");

    if documents.is_empty() {
        readme_content.push_str("_まだドキュメントはありません_\n");
    } else {
        readme_content.push_str("| ファイル | 生成日時 | AI Agent |\n");
        readme_content.push_str("|---------|---------|----------|\n");

        for (filename, generated_at, agent_role) in documents {
            readme_content.push_str(&format!(
                "| [{}](./{}) | {} | {} |\n",
                filename, filename, generated_at, agent_role
            ));
        }
    }

    readme_content.push_str("\n---\n\n");
    readme_content.push_str("_このREADME.mdは自動生成されています_\n");

    // README.mdを保存
    let readme_path = docs_dir.join("README.md");
    fs::write(&readme_path, readme_content)?;

    info!("Updated README.md: {:?}", readme_path);

    Ok(())
}

/// List project files request
#[derive(Debug, Deserialize)]
struct ListProjectFilesRequest {
    project_path: String,
}

/// Project file info
#[derive(Debug, Serialize)]
struct ProjectFileInfo {
    path: String,
    relative_path: String,
    is_dir: bool,
    extension: Option<String>,
}

/// List project files response
#[derive(Debug, Serialize)]
struct ListProjectFilesResponse {
    files: Vec<ProjectFileInfo>,
}

/// List project files handler - returns all source files in a project
async fn list_project_files_handler(
    Json(request): Json<ListProjectFilesRequest>,
) -> impl IntoResponse {
    info!("Listing files for project: {}", request.project_path);

    let project_root = std::path::PathBuf::from(&request.project_path);

    if !project_root.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Project not found: {}", request.project_path)
            }))
        ).into_response();
    }

    let mut files = Vec::new();

    // 除外するディレクトリ
    let exclude_dirs = vec![
        "target", "node_modules", ".git", "dist", "build",
        ".next", ".vscode", ".idea", "data", "static"
    ];

    // 含めるファイル拡張子
    let include_extensions = vec![
        "rs", "toml", "md", "js", "ts", "jsx", "tsx", "py",
        "go", "java", "cpp", "c", "h", "html", "css", "json", "yaml", "yml"
    ];

    fn walk_dir(
        dir: &std::path::Path,
        project_root: &std::path::Path,
        files: &mut Vec<ProjectFileInfo>,
        exclude_dirs: &[&str],
        include_extensions: &[&str],
    ) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

            // ディレクトリの場合
            if path.is_dir() {
                // 除外ディレクトリをスキップ
                if exclude_dirs.contains(&file_name) {
                    continue;
                }
                // 再帰的に探索
                walk_dir(&path, project_root, files, exclude_dirs, include_extensions)?;
            } else {
                // ファイルの場合、拡張子をチェック
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if include_extensions.contains(&ext) {
                        let relative_path = path.strip_prefix(project_root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();

                        files.push(ProjectFileInfo {
                            path: path.to_string_lossy().to_string(),
                            relative_path,
                            is_dir: false,
                            extension: Some(ext.to_string()),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    match walk_dir(&project_root, &project_root, &mut files, &exclude_dirs, &include_extensions) {
        Ok(_) => {
            // パスでソート
            files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

            info!("Found {} files in project", files.len());

            Json(ListProjectFilesResponse { files }).into_response()
        }
        Err(e) => {
            warn!("Failed to list project files: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to list files: {}", e)
                }))
            ).into_response()
        }
    }
}

/// Read project files request
#[derive(Debug, Deserialize)]
struct ReadProjectFilesRequest {
    file_paths: Vec<String>,
}

/// File content
#[derive(Debug, Serialize)]
struct FileContent {
    path: String,
    content: String,
    error: Option<String>,
}

/// Read project files response
#[derive(Debug, Serialize)]
struct ReadProjectFilesResponse {
    files: Vec<FileContent>,
}

/// Read multiple project files handler
async fn read_project_files_handler(
    Json(request): Json<ReadProjectFilesRequest>,
) -> impl IntoResponse {
    info!("Reading {} files", request.file_paths.len());

    let mut files = Vec::new();

    for file_path in request.file_paths {
        let path = std::path::PathBuf::from(&file_path);

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                files.push(FileContent {
                    path: file_path,
                    content,
                    error: None,
                });
            }
            Err(e) => {
                warn!("Failed to read file {}: {}", file_path, e);
                files.push(FileContent {
                    path: file_path.clone(),
                    content: String::new(),
                    error: Some(format!("Failed to read: {}", e)),
                });
            }
        }
    }

    Json(ReadProjectFilesResponse { files }).into_response()
}

// ========== Project Management Handlers ==========

/// Delete project from history
#[derive(Debug, Deserialize)]
struct DeleteProjectRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct DeleteProjectResponse {
    status: String,
}

async fn delete_project_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<DeleteProjectRequest>,
) -> impl IntoResponse {
    info!("Deleting project from history: {}", params.path);

    let mut pm = state.project_manager.lock().await;
    let path = std::path::PathBuf::from(&params.path);

    match pm.remove_project(&path) {
        Ok(_) => Json(DeleteProjectResponse {
            status: "deleted".to_string(),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to delete project: {}", e)
            })),
        )
            .into_response(),
    }
}

/// Add existing project
#[derive(Debug, Deserialize)]
struct AddProjectRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct AddProjectResponse {
    path: String,
    status: String,
}

async fn add_project_handler(
    State(state): State<AppState>,
    Json(payload): Json<AddProjectRequest>,
) -> impl IntoResponse {
    info!("Adding existing project: {}", payload.path);

    let mut pm = state.project_manager.lock().await;
    let path = std::path::PathBuf::from(&payload.path);

    // Check if path exists
    if !path.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("フォルダが見つかりません: {}", payload.path)})),
        )
            .into_response();
    }

    // Check if it's a directory
    if !path.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("指定されたパスはディレクトリではありません: {}", payload.path)})),
        )
            .into_response();
    }

    // Add to project manager
    match pm.add_or_update_project(path.clone()) {
        Ok(_) => Json(AddProjectResponse {
            path: path.to_string_lossy().to_string(),
            status: "added".to_string(),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("プロジェクトの登録に失敗しました: {}", e)})),
        )
            .into_response(),
    }
}

/// Create new project
#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    name: String,
    path: Option<String>,
    init_git: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CreateProjectResponse {
    path: String,
    status: String,
}

async fn create_project_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    info!("Creating new project: {}", payload.name);

    let mut pm = state.project_manager.lock().await;

    let path = payload
        .path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap().join(&payload.name));

    // Create directory
    if let Err(e) = std::fs::create_dir_all(&path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to create directory: {}", e)
            })),
        )
            .into_response();
    }

    // Initialize git if requested
    if payload.init_git.unwrap_or(true) {
        let output = std::process::Command::new("git")
            .args(&["init"])
            .current_dir(&path)
            .output();

        if let Err(e) = output {
            warn!("Failed to initialize git: {}", e);
        }
    }

    // Add to project manager
    match pm.add_or_update_project(path.clone()) {
        Ok(_) => Json(CreateProjectResponse {
            path: path.to_string_lossy().to_string(),
            status: "created".to_string(),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to register project: {}", e)
            })),
        )
            .into_response(),
    }
}

/// Clone repository
#[derive(Debug, Deserialize)]
struct CloneRepositoryRequest {
    url: String,
    dest: Option<String>,
}

#[derive(Debug, Serialize)]
struct CloneRepositoryResponse {
    path: String,
    status: String,
}

async fn clone_repository_handler(
    State(state): State<AppState>,
    Json(payload): Json<CloneRepositoryRequest>,
) -> impl IntoResponse {
    info!("Cloning repository: {}", payload.url);

    let mut pm = state.project_manager.lock().await;

    // Extract repository name from URL
    let repo_name = payload
        .url
        .split('/')
        .last()
        .and_then(|s| s.strip_suffix(".git").or(Some(s)))
        .unwrap_or("cloned-repo");

    let dest = payload
        .dest
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap().join(repo_name));

    // Clone repository using git
    let output = std::process::Command::new("git")
        .args(&["clone", &payload.url, dest.to_str().unwrap()])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            // Add to project manager
            match pm.add_or_update_project(dest.clone()) {
                Ok(_) => Json(CloneRepositoryResponse {
                    path: dest.to_string_lossy().to_string(),
                    status: "cloned".to_string(),
                })
                .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Failed to register project: {}", e)
                    })),
                )
                    .into_response(),
            }
        }
        Ok(output) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!(
                    "Git clone failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to execute git: {}", e)
            })),
        )
            .into_response(),
    }
}
