//! Task Runner API
//! Generic task runner for executing custom tasks (build, test, run, etc.)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TaskRunnerState {
    pub running_tasks: Arc<Mutex<HashMap<String, TaskExecution>>>,
}

impl TaskRunnerState {
    pub fn new() -> Self {
        Self {
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub label: String,
    pub r#type: String, // "shell", "process"
    pub command: String,
    pub args: Option<Vec<String>>,
    pub options: Option<TaskOptions>,
    pub problem_matcher: Option<String>,
    pub group: Option<String>, // "build", "test", "none"
    pub presentation: Option<PresentationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOptions {
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub shell: Option<ShellOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOptions {
    pub executable: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationOptions {
    pub echo: Option<bool>,
    pub reveal: Option<String>, // "always", "silent", "never"
    pub focus: Option<bool>,
    pub panel: Option<String>, // "shared", "dedicated", "new"
    pub show_reuse_message: Option<bool>,
    pub clear: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskExecution {
    pub task_id: String,
    pub task_name: String,
    pub status: String, // "running", "completed", "failed"
    pub output: String,
    pub exit_code: Option<i32>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RunTaskRequest {
    pub session_id: String,
    pub project_path: String,
    pub task: TaskDefinition,
}

#[derive(Debug, Serialize)]
pub struct RunTaskResponse {
    pub success: bool,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTaskOutputRequest {
    pub task_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetTaskOutputResponse {
    pub success: bool,
    pub execution: Option<TaskExecution>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksRequest {
    pub session_id: String,
    pub project_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TasksConfig {
    pub version: String,
    pub tasks: Vec<TaskDefinition>,
}

#[derive(Debug, Serialize)]
pub struct ListTasksResponse {
    pub success: bool,
    pub tasks: Vec<TaskDefinition>,
    pub error: Option<String>,
}

/// Run a task endpoint
pub async fn run_task(
    State(state): State<TaskRunnerState>,
    Json(request): Json<RunTaskRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        task = %request.task.label,
        "Running task"
    );

    let task_id = uuid::Uuid::new_v4().to_string();
    let task_name = request.task.label.clone();
    let task = request.task.clone();
    let project_path = request.project_path.clone();

    // Create task execution record
    let execution = TaskExecution {
        task_id: task_id.clone(),
        task_name: task_name.clone(),
        status: "running".to_string(),
        output: String::new(),
        exit_code: None,
        started_at: chrono::Utc::now(),
        completed_at: None,
    };

    // Store in running tasks
    {
        let mut running_tasks = state.running_tasks.lock().await;
        running_tasks.insert(task_id.clone(), execution);
    }

    // Spawn task in background
    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        let result = execute_task(&task, &project_path).await;

        // Update task execution
        let mut running_tasks = state_clone.running_tasks.lock().await;
        if let Some(execution) = running_tasks.get_mut(&task_id_clone) {
            execution.output = result.output;
            execution.exit_code = result.exit_code;
            execution.status = if result.success { "completed" } else { "failed" }.to_string();
            execution.completed_at = Some(chrono::Utc::now());
        }
    });

    (
        StatusCode::OK,
        Json(RunTaskResponse {
            success: true,
            task_id,
            message: format!("Task '{}' started", task_name),
        }),
    )
}

#[derive(Debug)]
struct TaskResult {
    success: bool,
    output: String,
    exit_code: Option<i32>,
}

async fn execute_task(task: &TaskDefinition, project_path: &str) -> TaskResult {
    let cwd = if let Some(ref options) = task.options {
        if let Some(ref custom_cwd) = options.cwd {
            PathBuf::from(project_path).join(custom_cwd)
        } else {
            PathBuf::from(project_path)
        }
    } else {
        PathBuf::from(project_path)
    };

    let mut command = if task.r#type == "shell" {
        // Use shell to execute command
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.args(&["/C", &task.command]);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(&["-c", &task.command]);
            c
        };

        cmd
    } else {
        // Execute as process
        let mut cmd = Command::new(&task.command);
        if let Some(ref args) = task.args {
            cmd.args(args);
        }
        cmd
    };

    command.current_dir(&cwd);

    // Set environment variables
    if let Some(ref options) = task.options {
        if let Some(ref env) = options.env {
            for (key, value) in env {
                command.env(key, value);
            }
        }
    }

    // Execute command
    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let full_output = format!("{}{}", stdout, stderr);
            let exit_code = output.status.code();

            TaskResult {
                success: output.status.success(),
                output: full_output,
                exit_code,
            }
        }
        Err(e) => TaskResult {
            success: false,
            output: format!("Failed to execute task: {}", e),
            exit_code: None,
        },
    }
}

/// Get task output endpoint
pub async fn get_task_output(
    State(state): State<TaskRunnerState>,
    Query(request): Query<GetTaskOutputRequest>,
) -> impl IntoResponse {
    let running_tasks = state.running_tasks.lock().await;

    if let Some(execution) = running_tasks.get(&request.task_id) {
        (
            StatusCode::OK,
            Json(GetTaskOutputResponse {
                success: true,
                execution: Some(execution.clone()),
                error: None,
            }),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(GetTaskOutputResponse {
                success: false,
                execution: None,
                error: Some(format!("Task not found: {}", request.task_id)),
            }),
        )
    }
}

/// List available tasks from tasks.json or berrycode.json
pub async fn list_tasks(
    Query(request): Query<ListTasksRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        "Listing tasks"
    );

    let project_path = PathBuf::from(&request.project_path);

    // Try to load tasks from .vscode/tasks.json
    let vscode_tasks_path = project_path.join(".vscode/tasks.json");
    if vscode_tasks_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&vscode_tasks_path).await {
            if let Ok(config) = serde_json::from_str::<TasksConfig>(&content) {
                return (
                    StatusCode::OK,
                    Json(ListTasksResponse {
                        success: true,
                        tasks: config.tasks,
                        error: None,
                    }),
                );
            }
        }
    }

    // Try to load tasks from berrycode.json
    let berrycode_path = project_path.join("berrycode.json");
    if berrycode_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&berrycode_path).await {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(tasks) = config.get("tasks") {
                    if let Ok(tasks) = serde_json::from_value::<Vec<TaskDefinition>>(tasks.clone()) {
                        return (
                            StatusCode::OK,
                            Json(ListTasksResponse {
                                success: true,
                                tasks,
                                error: None,
                            }),
                        );
                    }
                }
            }
        }
    }

    // Return default task templates
    let default_tasks = get_default_tasks();
    (
        StatusCode::OK,
        Json(ListTasksResponse {
            success: true,
            tasks: default_tasks,
            error: None,
        }),
    )
}

fn get_default_tasks() -> Vec<TaskDefinition> {
    vec![
        TaskDefinition {
            label: "Build".to_string(),
            r#type: "shell".to_string(),
            command: "npm run build".to_string(),
            args: None,
            options: None,
            problem_matcher: Some("$tsc".to_string()),
            group: Some("build".to_string()),
            presentation: Some(PresentationOptions {
                echo: Some(true),
                reveal: Some("always".to_string()),
                focus: Some(false),
                panel: Some("shared".to_string()),
                show_reuse_message: Some(true),
                clear: Some(false),
            }),
        },
        TaskDefinition {
            label: "Test".to_string(),
            r#type: "shell".to_string(),
            command: "npm test".to_string(),
            args: None,
            options: None,
            problem_matcher: None,
            group: Some("test".to_string()),
            presentation: Some(PresentationOptions {
                echo: Some(true),
                reveal: Some("always".to_string()),
                focus: Some(false),
                panel: Some("shared".to_string()),
                show_reuse_message: Some(true),
                clear: Some(false),
            }),
        },
        TaskDefinition {
            label: "Run".to_string(),
            r#type: "shell".to_string(),
            command: "npm start".to_string(),
            args: None,
            options: None,
            problem_matcher: None,
            group: Some("none".to_string()),
            presentation: Some(PresentationOptions {
                echo: Some(true),
                reveal: Some("always".to_string()),
                focus: Some(false),
                panel: Some("shared".to_string()),
                show_reuse_message: Some(true),
                clear: Some(false),
            }),
        },
    ]
}
