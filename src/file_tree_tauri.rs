//! File Tree Panel - Tauri Version
//! Uses native file system access via Tauri commands

use leptos::prelude::*;
use crate::tauri_bindings::{self, FileNode};
use leptos::task::spawn_local;
use crate::web_worker::{IndexerWorker, ProgressData};

/// ✅ VS Code Style: Icon component using codicon font (matching VS Code exactly)
#[component]
fn FileIcon(is_dir: bool, expanded: bool, name: String) -> impl IntoView {
    if is_dir {
        // VS Code folder style: just the folder icon, no chevron
        if expanded {
            view! {
                <i class="codicon codicon-folder-opened" style="margin-right: 4px; flex-shrink: 0; font-size: 16px; color: #DCAA6F;"></i>
            }.into_any()
        } else {
            view! {
                <i class="codicon codicon-folder" style="margin-right: 4px; flex-shrink: 0; font-size: 16px; color: #C5C5C5;"></i>
            }.into_any()
        }
    } else {
        // VS Code file icons
        let extension = name.split('.').last().unwrap_or("");

        // Check for .disabled suffix
        let is_disabled = name.ends_with(".disabled");

        let (icon_class, color) = if is_disabled {
            ("exclude", "#C5C5C5")  // VS Code uses exclude icon for disabled files
        } else {
            match extension {
                "rs" => ("file", "#C5C5C5"),                    // Rust - simple file icon
                "toml" => ("settings-gear", "#C5C5C5"),         // Config - gear
                "md" => ("markdown", "#519ABA"),                // Markdown - blue
                "js" => ("symbol-misc", "#F1DD3F"),             // JavaScript - yellow
                "ts" | "tsx" => ("symbol-misc", "#3B8AD8"),     // TypeScript - blue
                "jsx" => ("symbol-misc", "#61DAFB"),            // JSX - cyan
                "html" => ("file-code", "#E37933"),             // HTML - orange
                "css" | "scss" | "sass" => ("symbol-misc", "#519ABA"), // CSS - blue
                "json" => ("json", "#FBC02D"),                  // JSON - yellow
                "yaml" | "yml" => ("gear", "#CB4335"),          // YAML - red
                "xml" => ("file-code", "#E37933"),              // XML - orange
                "py" => ("symbol-misc", "#3776AB"),             // Python - blue
                "java" => ("symbol-misc", "#EA2D2E"),           // Java - red
                "go" => ("symbol-misc", "#00ADD8"),             // Go - cyan
                "sh" | "bash" => ("terminal-bash", "#89E051"), // Shell - green
                "lock" => ("lock", "#C5C5C5"),                  // Lock files
                _ => ("file", "#C5C5C5"),                       // Default - gray file icon
            }
        };

        view! {
            <i class=format!("codicon codicon-{}", icon_class) style=format!("margin-right: 4px; flex-shrink: 0; font-size: 16px; color: {};", color)></i>
        }.into_any()
    }
}

#[component]
pub fn FileTreePanelTauri(
    on_file_select: RwSignal<Option<(String, String)>>,
    root_path: String,
) -> impl IntoView {
    let tree = RwSignal::new(Vec::<FileNode>::new());
    let is_loading = RwSignal::new(true);

    // ✅ IntelliJ Pro: Symbol indexing state
    let is_indexing = RwSignal::new(false);
    let symbol_count = RwSignal::new(0_usize);


    // CRITICAL: Load immediately in component body, not in Effect

    // ✅ In test environment, skip Tauri backend calls and show empty tree
    #[cfg(test)]
    {
        tree.set(Vec::new());
        is_loading.set(false);
    }

    // ✅ Only call Tauri backend in non-test environment
    #[cfg(not(test))]
    {
        let root_for_tree = root_path.clone();
        spawn_local(async move {
            // ✅ IntelliJ Design: Lazy Loading - load only first level initially
            // Further levels are loaded on-demand when folders are expanded
            match tauri_bindings::read_dir(&root_for_tree, Some(1)).await {
                Ok(nodes) => {
                    // ✅ Safe: Use .set() to trigger reactivity and update UI
                    tree.set(nodes);
                    is_loading.set(false);
                }
                Err(e) => {
                    // ✅ Safe: set empty on error
                    tree.set(Vec::new());
                    is_loading.set(false);
                }
            }
        });
    }

    // ✅ IntelliJ Pro: Index workspace on button click
    let on_index_click = move |_| {
        let root = root_path.clone();
        is_indexing.set(true);

        spawn_local(async move {

            match tauri_bindings::index_workspace(&root).await {
                Ok(count) => {
                    // ✅ Safe: Use .set() to update UI
                    symbol_count.set(count);
                    is_indexing.set(false);
                }
                Err(e) => {
                    // ✅ Safe: set on error
                    is_indexing.set(false);
                }
            }
        });
    };

    view! {
        <div class="berry-editor-sidebar" style="background: #1E1F22 !important; border-right: 1px solid #1E1F22 !important;">
            <div class="berry-editor-sidebar-header">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                    <span>"EXPLORER"</span>

                    // ✅ IntelliJ Pro: Index workspace button
                    <button
                        on:click=on_index_click
                        disabled=move || is_indexing.get()
                        style="
                            font-size: 11px;
                            padding: 2px 6px;
                            background: #1E1F22;
                            border: 1px solid #1E1F22;
                            color: #cccccc;
                            cursor: pointer;
                            border-radius: 3px;
                        "
                        title="Index workspace for symbol search"
                    >
                        {move || {
                            if is_indexing.get() {
                                "Indexing..."
                            } else {
                                "Index"
                            }
                        }}
                    </button>
                </div>

                // ✅ IntelliJ Pro: Symbol count display
                {move || {
                    let count = symbol_count.get();
                    if count > 0 {
                        view! {
                            <div style="font-size: 10px; color: #858585; margin-top: 4px;">
                                {format!("{} symbols indexed", count)}
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>
            <div class="berry-editor-file-tree" style="background: #1E1F22 !important;">
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
    // ✅ Make node reactive to update when children are loaded
    let node_signal = RwSignal::new(node);
    let expanded = RwSignal::new(false);
    let is_loading_children = RwSignal::new(false);
    let indent = (level * 16) + 8;

    view! {
        <div>
            <div
                class="berry-editor-file-item"
                style:padding-left=format!("{}px", indent)
                on:click=move |_| {
                    let current_node = node_signal.get_untracked();

                    if current_node.is_dir {
                        // Toggle folder expansion
                        if !expanded.get_untracked() {
                            // Opening folder - check if we need to load children
                            if current_node.children.is_none() {
                                // ✅ IntelliJ Design: On-demand loading
                                is_loading_children.set(true);
                                let path = current_node.path.clone();

                                spawn_local(async move {
                                    // Load only first level (depth=1) for memory efficiency
                                    match tauri_bindings::read_dir(&path, Some(1)).await {
                                        Ok(children) => {
                                            // ✅ Safe: Update node and UI
                                            node_signal.update(|n| n.children = Some(children));
                                            is_loading_children.set(false);
                                            expanded.set(true);
                                        }
                                        Err(_) => {
                                            // ✅ Safe: set on error
                                            is_loading_children.set(false);
                                        }
                                    }
                                });
                            } else {
                                // Children already loaded, just expand
                                expanded.set(true);
                            }
                        } else {
                            // Closing folder
                            expanded.set(false);
                        }
                    } else {
                        // File clicked - load content via Tauri
                        let path = current_node.path.clone();

                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::prelude::*;
                            #[wasm_bindgen]
                            extern "C" {
                                #[wasm_bindgen(js_namespace = console)]
                                fn log(s: &str);
                            }
                            log(&format!("File clicked: {}", path));
                        }

                        spawn_local(async move {
                            #[cfg(target_arch = "wasm32")]
                            {
                                use wasm_bindgen::prelude::*;
                                #[wasm_bindgen]
                                extern "C" {
                                    #[wasm_bindgen(js_namespace = console)]
                                    fn log(s: &str);
                                }
                                log(&format!("spawn_local started for: {}", path));
                            }

                            match tauri_bindings::read_file(&path).await {
                                Ok(content) => {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        use wasm_bindgen::prelude::*;
                                        #[wasm_bindgen]
                                        extern "C" {
                                            #[wasm_bindgen(js_namespace = console)]
                                            fn log(s: &str);
                                        }
                                        log(&format!("File read success: {}, length: {}", path, content.len()));
                                    }
                                    // ✅ Safe: Update file selection
                                    on_file_select.set(Some((path.clone(), content)));
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        use wasm_bindgen::prelude::*;
                                        #[wasm_bindgen]
                                        extern "C" {
                                            #[wasm_bindgen(js_namespace = console)]
                                            fn log(s: &str);
                                        }
                                        log(&format!("on_file_select.set called for: {}", path));
                                    }
                                }
                                Err(e) => {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        use wasm_bindgen::prelude::*;
                                        #[wasm_bindgen]
                                        extern "C" {
                                            #[wasm_bindgen(js_namespace = console)]
                                            fn log(s: &str);
                                        }
                                        log(&format!("File read error: {}, error: {}", path, e));
                                    }
                                    let error_content = format!("// Error loading file: {}\n// {}", path, e);
                                    // ✅ Safe: set error content
                                    on_file_select.set(Some((path, error_content)));
                                }
                            }
                        });
                    }
                }
            >
                {move || {
                    let current_node = node_signal.get();
                    view! {
                        <FileIcon
                            is_dir=current_node.is_dir
                            expanded=expanded.get()
                            name=current_node.name.clone()
                        />
                        <span>{current_node.name.clone()}</span>
                    }
                }}
            </div>
            {move || {
                let current_node = node_signal.get();
                if current_node.is_dir && expanded.get() {
                    if let Some(ref children) = current_node.children {
                        children.iter().map(|child| {
                            view! {
                                <FileTreeNodeTauri node=child.clone() level=level + 1 on_file_select=on_file_select />
                            }
                        }).collect_view().into_any()
                    } else if is_loading_children.get() {
                        view! {
                            <div style=format!("padding-left: {}px; color: #858585; font-size: 11px;", indent + 16)>
                                "Loading..."
                            </div>
                        }.into_any()
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
