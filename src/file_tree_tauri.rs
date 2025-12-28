//! File Tree Panel - Tauri Version
//! Uses native file system access via Tauri commands

use leptos::prelude::*;
use crate::tauri_bindings::{self, FileNode};
use leptos::task::spawn_local;
use crate::web_worker::{IndexerWorker, ProgressData};

/// ✅ IntelliJ Pattern: SVG-based file/folder icon component (Flat Design)
#[component]
fn FileIcon(is_dir: bool, expanded: bool, name: String) -> impl IntoView {
    if is_dir {
        // IntelliJ風のフラットフォルダアイコン（展開/非展開）
        if expanded {
            view! {
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right: 6px; flex-shrink: 0;">
                    <path d="M1.5 3C1.22 3 1 3.22 1 3.5V4.5H15V6C15 6.55 14.55 7 14 7H2V13C2 13.55 2.45 14 3 14H13C13.55 14 14 13.55 14 13V7H14.5C14.78 7 15 6.78 15 6.5V4C15 3.45 14.55 3 14 3H7.5L6.5 2H1.5V3Z" fill="#6E9ECF"/>
                    <path d="M1 4.5H7L8 3.5H14.5C14.78 3.5 15 3.72 15 4V6H2V13.5C2 13.78 2.22 14 2.5 14H13.5C13.78 14 14 13.78 14 13.5V7H14.5" stroke="#5A8AC4" stroke-width="0.5" opacity="0.5"/>
                </svg>
            }.into_any()
        } else {
            view! {
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right: 6px; flex-shrink: 0;">
                    <path d="M1.5 3C1.22 3 1 3.22 1 3.5V13C1 13.55 1.45 14 2 14H14C14.55 14 15 13.55 15 13V5C15 4.45 14.55 4 14 4H7L6 3H1.5Z" fill="#9AA7B0"/>
                    <path d="M1.5 3H6L7 4H14C14.55 4 15 4.45 15 5V13C15 13.55 14.55 14 14 14H2C1.45 14 1 13.55 1 13V3.5C1 3.22 1.22 3 1.5 3Z" stroke="#7D8A94" stroke-width="0.5" opacity="0.4"/>
                </svg>
            }.into_any()
        }
    } else {
        // 拡張子に応じたIntelliJ風のフラットファイルアイコン
        let extension = name.split('.').last().unwrap_or("");
        let (bg_color, badge_color) = match extension {
            "rs" => ("#6C707E", "#E44D26"),      // Rust - 赤バッジ
            "toml" => ("#6C707E", "#9C9C9C"),    // Config - グレーバッジ
            "md" => ("#6C707E", "#4A90E2"),      // Markdown - 青バッジ
            "js" => ("#6C707E", "#F7DF1E"),      // JavaScript - 黄バッジ
            "ts" => ("#6C707E", "#3178C6"),      // TypeScript - 青バッジ
            "tsx" => ("#6C707E", "#3178C6"),     // TSX - 青バッジ
            "jsx" => ("#6C707E", "#61DAFB"),     // JSX - シアンバッジ
            "html" => ("#6C707E", "#E34F26"),    // HTML - オレンジバッジ
            "css" => ("#6C707E", "#1572B6"),     // CSS - 青バッジ
            "scss" | "sass" => ("#6C707E", "#CD6799"), // Sass - ピンクバッジ
            "json" => ("#6C707E", "#5E97D0"),    // JSON - 青バッジ
            "yaml" | "yml" => ("#6C707E", "#CB4335"), // YAML - 赤バッジ
            "xml" => ("#6C707E", "#E37933"),     // XML - オレンジバッジ
            "py" => ("#6C707E", "#3776AB"),      // Python - 青バッジ
            "java" => ("#6C707E", "#EA2D2E"),    // Java - 赤バッジ
            "go" => ("#6C707E", "#00ADD8"),      // Go - シアンバッジ
            "sh" | "bash" => ("#6C707E", "#89E051"), // Shell - 緑バッジ
            _ => ("#6C707E", "#AFB1B3"),         // Default - グレーバッジ
        };

        view! {
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right: 6px; flex-shrink: 0;">
                // Base file shape
                <path d="M3 1C2.45 1 2 1.45 2 2V14C2 14.55 2.45 15 3 15H13C13.55 15 14 14.55 14 14V5L10 1H3Z" fill=bg_color/>
                <path d="M10 1V4C10 4.55 10.45 5 11 5H14" fill="#5A5D6B"/>
                // Extension badge
                <rect x="2.5" y="11" width="11" height="3" rx="0.5" fill=badge_color opacity="0.9"/>
            </svg>
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
        <div class="berry-editor-sidebar">
            <div class="berry-editor-sidebar-header">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                    <span>"PROJECT"</span>

                    // ✅ IntelliJ Pro: Index workspace button
                    <button
                        on:click=on_index_click
                        disabled=move || is_indexing.get()
                        style="
                            font-size: 11px;
                            padding: 2px 6px;
                            background: #2d2d2d;
                            border: 1px solid #3e3e3e;
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
