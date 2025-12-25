//! Hierarchical File Tree Component
//! 100% Rust - No JavaScript!

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Request, RequestInit, RequestMode, Response};
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

async fn fetch_file_tree(session_id: &str) -> Result<Vec<FileNode>, String> {
    let url = format!("/api/files/tree?session_id={}", session_id);

    let mut opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let window = window().ok_or("no window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(
        resp.json()
            .map_err(|e| format!("Failed to get json: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Failed to deserialize: {}", e))
}

fn get_session_id() -> Option<String> {
    let window = window()?;
    let location = window.location();
    let pathname = location.pathname().ok()?;

    // Extract session_id from /chat/{session_id} pattern
    let parts: Vec<&str> = pathname.split('/').collect();
    if parts.len() >= 3 && parts[1] == "chat" {
        Some(parts[2].to_string())
    } else {
        None
    }
}

#[component]
fn FileTreeNode(node: FileNode, level: usize) -> impl IntoView {
    let (expanded, set_expanded) = signal(false);
    let indent = level * 16;

    let icon = if node.is_dir {
        if expanded.get() { "📂" } else { "📁" }
    } else {
        match node.path.split('.').last() {
            Some("rs") => "🦀",
            Some("js") | Some("ts") => "📜",
            Some("html") => "🌐",
            Some("css") => "🎨",
            Some("md") => "📝",
            _ => "📄",
        }
    };

    view! {
        <div class="file-tree-node">
            <div
                class="file-tree-item"
                style:padding-left=format!("{}px", indent)
                on:click=move |_| {
                    if node.is_dir {
                        set_expanded.update(|e| *e = !*e);
                    }
                }
            >
                <span class="file-icon">{icon}</span>
                <span class="file-name">{node.name.clone()}</span>
            </div>
            {move || {
                if node.is_dir && expanded.get() {
                    if let Some(children) = &node.children {
                        children.iter().map(|child| {
                            view! {
                                <FileTreeNode node=child.clone() level=level + 1 />
                            }
                        }).collect::<Vec<_>>().into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}

#[component]
pub fn FileTreePanel() -> impl IntoView {
    let (tree, set_tree) = signal(Vec::<FileNode>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);

    // Load file tree on mount
    Effect::new(move || {
        if let Some(session_id) = get_session_id() {
            spawn_local(async move {
                match fetch_file_tree(&session_id).await {
                    Ok(nodes) => {
                        set_tree.set(nodes);
                        set_loading.set(false);
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Failed to load file tree: {}", e)));
                        set_loading.set(false);
                    }
                }
            });
        } else {
            set_error.set(Some("No session ID found".to_string()));
            set_loading.set(false);
        }
    });

    view! {
        <div class="berry-editor-sidebar">
            <div class="berry-editor-sidebar-header">
                <span>"FILE EXPLORER"</span>
                <button
                    class="berry-editor-refresh-btn"
                    on:click=move |_| {
                        set_loading.set(true);
                        set_error.set(None);
                        if let Some(session_id) = get_session_id() {
                            spawn_local(async move {
                                match fetch_file_tree(&session_id).await {
                                    Ok(nodes) => {
                                        set_tree.set(nodes);
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        set_error.set(Some(format!("Failed to load: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            });
                        }
                    }
                >
                    "⟳"
                </button>
            </div>
            <div class="berry-editor-file-tree">
                {move || {
                    if loading.get() {
                        view! {
                            <div style="padding:16px;color:#858585;font-size:12px;">
                                "Loading..."
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div style="padding:16px;color:#f48771;font-size:12px;">
                                {err}
                            </div>
                        }.into_any()
                    } else {
                        tree.get().iter().map(|node| {
                            view! {
                                <FileTreeNode node=node.clone() level=0 />
                            }
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_file_node_creation() {
        let node = FileNode {
            name: "main.rs".to_string(),
            path: "/src/main.rs".to_string(),
            is_dir: false,
            children: None,
        };

        assert_eq!(node.name, "main.rs");
        assert_eq!(node.path, "/src/main.rs");
        assert!(!node.is_dir);
        assert!(node.children.is_none());
    }

    #[wasm_bindgen_test]
    fn test_directory_node_creation() {
        let children = vec![
            FileNode {
                name: "main.rs".to_string(),
                path: "/src/main.rs".to_string(),
                is_dir: false,
                children: None,
            },
            FileNode {
                name: "lib.rs".to_string(),
                path: "/src/lib.rs".to_string(),
                is_dir: false,
                children: None,
            },
        ];

        let dir_node = FileNode {
            name: "src".to_string(),
            path: "/src".to_string(),
            is_dir: true,
            children: Some(children),
        };

        assert_eq!(dir_node.name, "src");
        assert!(dir_node.is_dir);
        assert!(dir_node.children.is_some());
        assert_eq!(dir_node.children.as_ref().unwrap().len(), 2);
    }

    #[wasm_bindgen_test]
    fn test_file_node_serialization() {
        let node = FileNode {
            name: "test.rs".to_string(),
            path: "/test.rs".to_string(),
            is_dir: false,
            children: None,
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"name\":\"test.rs\""));
        assert!(json.contains("\"is_dir\":false"));
    }

    #[wasm_bindgen_test]
    fn test_file_node_deserialization() {
        let json = r#"{
            "name": "main.rs",
            "path": "/src/main.rs",
            "is_dir": false,
            "children": null
        }"#;

        let node: FileNode = serde_json::from_str(json).unwrap();
        assert_eq!(node.name, "main.rs");
        assert_eq!(node.path, "/src/main.rs");
        assert!(!node.is_dir);
        assert!(node.children.is_none());
    }

    #[wasm_bindgen_test]
    fn test_nested_directory_structure() {
        let nested_file = FileNode {
            name: "nested.rs".to_string(),
            path: "/src/utils/nested.rs".to_string(),
            is_dir: false,
            children: None,
        };

        let utils_dir = FileNode {
            name: "utils".to_string(),
            path: "/src/utils".to_string(),
            is_dir: true,
            children: Some(vec![nested_file]),
        };

        let main_file = FileNode {
            name: "main.rs".to_string(),
            path: "/src/main.rs".to_string(),
            is_dir: false,
            children: None,
        };

        let src_dir = FileNode {
            name: "src".to_string(),
            path: "/src".to_string(),
            is_dir: true,
            children: Some(vec![main_file, utils_dir]),
        };

        assert!(src_dir.is_dir);
        assert_eq!(src_dir.children.as_ref().unwrap().len(), 2);

        let utils = &src_dir.children.as_ref().unwrap()[1];
        assert!(utils.is_dir);
        assert_eq!(utils.children.as_ref().unwrap().len(), 1);
    }

    #[wasm_bindgen_test]
    fn test_empty_directory() {
        let empty_dir = FileNode {
            name: "empty".to_string(),
            path: "/empty".to_string(),
            is_dir: true,
            children: Some(Vec::new()),
        };

        assert!(empty_dir.is_dir);
        assert!(empty_dir.children.is_some());
        assert_eq!(empty_dir.children.as_ref().unwrap().len(), 0);
    }

    #[wasm_bindgen_test]
    fn test_file_tree_json_roundtrip() {
        let original = vec![
            FileNode {
                name: "README.md".to_string(),
                path: "/README.md".to_string(),
                is_dir: false,
                children: None,
            },
            FileNode {
                name: "src".to_string(),
                path: "/src".to_string(),
                is_dir: true,
                children: Some(vec![FileNode {
                    name: "main.rs".to_string(),
                    path: "/src/main.rs".to_string(),
                    is_dir: false,
                    children: None,
                }]),
            },
        ];

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Vec<FileNode> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].name, "README.md");
        assert_eq!(deserialized[1].name, "src");
        assert!(deserialized[1].is_dir);
    }

    #[wasm_bindgen_test]
    fn test_file_extensions() {
        let rust_file = FileNode {
            name: "main.rs".to_string(),
            path: "/main.rs".to_string(),
            is_dir: false,
            children: None,
        };

        let js_file = FileNode {
            name: "index.js".to_string(),
            path: "/index.js".to_string(),
            is_dir: false,
            children: None,
        };

        let html_file = FileNode {
            name: "index.html".to_string(),
            path: "/index.html".to_string(),
            is_dir: false,
            children: None,
        };

        assert!(rust_file.name.ends_with(".rs"));
        assert!(js_file.name.ends_with(".js"));
        assert!(html_file.name.ends_with(".html"));
    }
}
