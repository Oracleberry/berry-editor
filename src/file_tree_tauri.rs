//! File Tree Panel - Tauri Version
//! Uses native file system access via Tauri commands

use leptos::prelude::*;
use crate::tauri_bindings::{self, FileNode};
use leptos::task::spawn_local;

#[component]
pub fn FileTreePanelTauri(
    on_file_select: RwSignal<Option<(String, String)>>,
    root_path: String,
) -> impl IntoView {
    let tree = RwSignal::new(Vec::<FileNode>::new());
    let is_loading = RwSignal::new(true);

    web_sys::console::log_1(&format!("[FileTreeTauri] Mounting with root: {}", root_path).into());

    // CRITICAL: Load immediately in component body, not in Effect
    web_sys::console::log_1(&"[FileTreeTauri] Starting direct spawn_local...".into());

    spawn_local(async move {
        web_sys::console::log_1(&format!("[FileTreeTauri] Async task started for: {}", root_path).into());

        match tauri_bindings::read_dir(&root_path, Some(3)).await {
            Ok(nodes) => {
                web_sys::console::log_1(&format!("[FileTreeTauri] Loaded {} nodes", nodes.len()).into());
                tree.set(nodes);
                is_loading.set(false);
                web_sys::console::log_1(&"[FileTreeTauri] Signals updated!".into());
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[FileTreeTauri] Error loading dir: {}", e).into());
                tree.set(Vec::new());
                is_loading.set(false);
            }
        }
    });

    view! {
        <div class="berry-editor-sidebar">
            <div class="berry-editor-sidebar-header">
                "PROJECT"
            </div>
            <div class="berry-editor-file-tree">
                {move || {
                    if is_loading.get() {
                        view! {
                            <div style="padding: 10px; color: #858585;">
                                "Loading files..."
                            </div>
                        }.into_any()
                    } else {
                        let nodes = tree.get();
                        if nodes.is_empty() {
                            view! {
                                <div style="padding: 10px; color: #858585;">
                                    "No files found"
                                </div>
                            }.into_any()
                        } else {
                            nodes.iter().map(|node| {
                                view! {
                                    <FileTreeNodeTauri node=node.clone() level=0 on_file_select=on_file_select />
                                }
                            }).collect_view().into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn FileTreeNodeTauri(
    node: FileNode,
    level: usize,
    on_file_select: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let indent = (level * 16) + 8;

    let node_clone = node.clone();
    let node_for_icon = node.clone();
    let node_for_children = node.clone();
    let path_for_click = node.path.clone();

    view! {
        <div>
            <div
                class="berry-editor-file-item"
                style:padding-left=format!("{}px", indent)
                on:click=move |_| {
                    web_sys::console::log_1(&format!("[FileTreeNode] Clicked: {} (is_dir={})", node_clone.name, node_clone.is_dir).into());

                    if node_clone.is_dir {
                        web_sys::console::log_1(&"[FileTreeNode] Toggling folder".into());
                        expanded.update(|e| *e = !*e);
                    } else {
                        // File clicked - load content via Tauri
                        web_sys::console::log_1(&format!("[FileTreeNode] Loading file: {}", path_for_click).into());

                        let path = path_for_click.clone();
                        spawn_local(async move {
                            match tauri_bindings::read_file(&path).await {
                                Ok(content) => {
                                    web_sys::console::log_1(&format!("[FileTreeNode] Loaded {} bytes", content.len()).into());
                                    on_file_select.set(Some((path, content)));
                                }
                                Err(e) => {
                                    web_sys::console::log_1(&format!("[FileTreeNode] Error loading file: {}", e).into());
                                    let error_content = format!("// Error loading file: {}\n// {}", path, e);
                                    on_file_select.set(Some((path, error_content)));
                                }
                            }
                        });
                    }
                }
            >
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
                                Some("toml") => "⚙️",
                                Some("json") => "📋",
                                _ => "📄",
                            }
                        }
                    }}
                </span>
                <span>{node.name.clone()}</span>
            </div>
            {move || {
                if node_for_children.is_dir && expanded.get() {
                    if let Some(ref children) = node_for_children.children {
                        children.iter().map(|child| {
                            view! {
                                <FileTreeNodeTauri node=child.clone() level=level + 1 on_file_select=on_file_select />
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
