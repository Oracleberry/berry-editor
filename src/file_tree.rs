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

pub fn get_mock_file_tree() -> Vec<FileNode> {
    vec![
        FileNode {
            name: "src".to_string(),
            path: "/src".to_string(),
            is_dir: true,
            children: Some(vec![
                FileNode {
                    name: "lib.rs".to_string(),
                    path: "/src/lib.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "components.rs".to_string(),
                    path: "/src/components.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "editor.rs".to_string(),
                    path: "/src/editor.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "file_tree.rs".to_string(),
                    path: "/src/file_tree.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "buffer.rs".to_string(),
                    path: "/src/buffer.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "syntax.rs".to_string(),
                    path: "/src/syntax.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "lsp.rs".to_string(),
                    path: "/src/lsp.rs".to_string(),
                    is_dir: false,
                    children: None,
                },
                FileNode {
                    name: "debugger".to_string(),
                    path: "/src/debugger".to_string(),
                    is_dir: true,
                    children: Some(vec![
                        FileNode {
                            name: "mod.rs".to_string(),
                            path: "/src/debugger/mod.rs".to_string(),
                            is_dir: false,
                            children: None,
                        },
                        FileNode {
                            name: "session.rs".to_string(),
                            path: "/src/debugger/session.rs".to_string(),
                            is_dir: false,
                            children: None,
                        },
                    ]),
                },
            ]),
        },
        FileNode {
            name: "Cargo.toml".to_string(),
            path: "/Cargo.toml".to_string(),
            is_dir: false,
            children: None,
        },
        FileNode {
            name: "index.html".to_string(),
            path: "/index.html".to_string(),
            is_dir: false,
            children: None,
        },
        FileNode {
            name: "README.md".to_string(),
            path: "/README.md".to_string(),
            is_dir: false,
            children: None,
        },
    ]
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

async fn fetch_file_content(session_id: &str, file_path: &str) -> Result<String, String> {
    let url = format!("/api/files/read?session_id={}&path={}",
        session_id,
        urlencoding::encode(file_path)
    );

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

    let text = JsFuture::from(
        resp.text()
            .map_err(|e| format!("Failed to get text: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to parse text: {:?}", e))?;

    text.as_string().ok_or_else(|| "Response is not a string".to_string())
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
fn FileTreeNode(
    node: FileNode,
    level: usize,
    on_file_select: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let indent_step = 12;
    let base_padding = 8;
    
    // Generate tree lines for visual hierarchy
    let tree_lines = (1..=level).map(|l| {
        let left = base_padding + (l - 1) * indent_step + 4;
        view! {
            <div class="berry-editor-tree-line" style:left=format!("{}px", left)></div>
        }
    }).collect_view();

    let node_clone = node.clone();
    let node_for_icon = node.clone();

    view! {
        <div>
            <div
                class="berry-editor-file-item"
                style:padding-left=format!("{}px", base_padding + level * indent_step)
                on:click=move |_| {

                    if node_clone.is_dir {
                        expanded.update(|e| *e = !*e);
                    } else {
                        // File clicked - load content

                        let path = node_clone.path.clone();
                        let name = node_clone.name.clone();

                        if let Some(session_id) = get_session_id() {
                            // Session mode: fetch real file content from API
                            spawn_local(async move {
                                match fetch_file_content(&session_id, &path).await {
                                    Ok(content) => {
                                        on_file_select.set(Some((path, content)));
                                    }
                                    Err(e) => {
                                        let error_content = format!("// Error loading file: {}\n// {}", path, e);
                                        on_file_select.set(Some((path, error_content)));
                                    }
                                }
                            });
                        } else {
                            // Standalone mode: use mock content
                            let mock_content = format!(
                                "// {}\n// This is a placeholder for file content\n\nfn main() {{\n    println!(\"File: {}\");\n}}",
                                path,
                                name
                            );
                            on_file_select.set(Some((path, mock_content)));
                        }

                    }
                }
            >
                {tree_lines}
                <span class="berry-editor-folder-icon"
                    class:expanded=move || expanded.get()
                    style:visibility=if node.is_dir { "visible" } else { "hidden" }
                >
                    "▶"
                </span>
                <span class="berry-editor-file-icon">
                    {move || {
                        if node_for_icon.is_dir {
                            if expanded.get() { "📂" } else { "📁" }
                        } else {
                            match node_for_icon.path.split('.').last() {
                                Some("rs") => "🦀",
                                Some("js") | Some("ts") => "📜",
                                Some("html") => "🌐",
                                Some("css") => "🎨",
                                Some("md") => "📝",
                                _ => "📄",
                            }
                        }
                    }}
                </span>
                <span>{node.name.clone()}</span>
            </div>
            {move || {
                if node.is_dir && expanded.get() {
                    if let Some(children) = &node.children {
                        children.iter().map(|child| {
                            view! {
                                <FileTreeNode node=child.clone() level=level + 1 on_file_select=on_file_select />
                            }
                        }).collect_view().into_any()
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
pub fn FileTreePanel(
    on_file_select: RwSignal<Option<(String, String)>>,
) -> impl IntoView {

    // Check session mode
    let session_id = get_session_id();

    // Initialize with mock data immediately for standalone mode
    let initial_data = if session_id.is_none() {
        let mock_data = get_mock_file_tree();
        mock_data
    } else {
        Vec::new()
    };

    let tree = RwSignal::new(initial_data);
    let loading = RwSignal::new(session_id.is_some());
    let error = RwSignal::new(Option::<String>::None);


    // Load from API only in session mode
    if let Some(sid) = session_id {
        spawn_local(async move {
            match fetch_file_tree(&sid).await {
                Ok(nodes) => {
                    tree.set(nodes);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load file tree: {}", e)));
                    loading.set(false);
                }
            }
        });
    }


    view! {
        <div class="berry-editor-sidebar" style="border: 5px solid green; background: #1e1e1e; width: 250px; height: 100vh;">
            <div class="berry-editor-sidebar-header" style="border: 2px solid yellow; padding: 8px; background: #252526; color: #cccccc; display: flex; justify-content: space-between; align-items: center;">
                <span>"FILE EXPLORER"</span>
                <button
                    class="berry-editor-refresh-btn"
                    style="background: none; border: none; color: #cccccc; cursor: pointer; font-size: 16px;"
                    on:click=move |_| {
                        loading.set(true);
                        error.set(None);
                        if let Some(session_id) = get_session_id() {
                            spawn_local(async move {
                                match fetch_file_tree(&session_id).await {
                                    Ok(nodes) => {
                                        tree.set(nodes);
                                        loading.set(false);
                                    }
                                    Err(e) => {
                                        error.set(Some(format!("Failed to load: {}", e)));
                                        loading.set(false);
                                    }
                                }
                            });
                        } else {
                            let mock_data = get_mock_file_tree();
                            tree.set(mock_data);
                            loading.set(false);
                        }
                    }
                >
                    "⟳"
                </button>
            </div>

            <div class="berry-editor-file-tree" style="border: 2px solid orange; overflow-y: auto; height: calc(100vh - 48px);">
                {
                    // 初期レンダリング時に即座に実行
                    let nodes = tree.get_untracked();

                    nodes.into_iter().enumerate().map(|(idx, node)| {
                        view! {
                            <FileTreeNode node=node level=0 on_file_select=on_file_select />
                        }
                    }).collect_view()
                }
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

    #[wasm_bindgen_test]
    fn test_get_mock_file_tree_contains_files() {
        let tree = get_mock_file_tree();

        // Should have 4 root items
        assert_eq!(tree.len(), 4, "Should have 4 root items");

        // Check each item
        assert_eq!(tree[0].name, "src");
        assert!(tree[0].is_dir);
        assert!(tree[0].children.is_some());

        assert_eq!(tree[1].name, "Cargo.toml");
        assert!(!tree[1].is_dir);

        assert_eq!(tree[2].name, "index.html");
        assert!(!tree[2].is_dir);

        assert_eq!(tree[3].name, "README.md");
        assert!(!tree[3].is_dir);

        // Check src children
        let src_children = tree[0].children.as_ref().unwrap();
        assert!(src_children.len() >= 7, "src should have at least 7 children");
    }
}

