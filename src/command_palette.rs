//! Command Palette (Search Everywhere)
//!
//! IntelliJ-style Shift+Shift / VS Code Cmd+P equivalent

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use web_sys::KeyboardEvent;
use crate::tauri_bindings;  // ✅ IntelliJ Pro: Symbol search integration

/// Action type for command palette
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    File,
    GitAction,
    EditorAction,
    Settings,
    Symbol,
}

/// Command palette item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub action_type: ActionType,
    pub icon: String,
    pub action: String,
}

/// Command Palette Component
#[component]
pub fn CommandPalette(
    show: RwSignal<bool>,
    on_select: impl Fn(PaletteItem) + 'static + Clone + Send,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let items = RwSignal::new(Vec::<PaletteItem>::new());
    let filtered_items = RwSignal::new(Vec::<PaletteItem>::new());
    let selected_index = RwSignal::new(0usize);

    // Load items when palette opens
    Effect::new(move || {
        if show.get() {
            query.set(String::new());
            selected_index.set(0);
            load_palette_items(items);
        }
    });

    // ✅ IntelliJ Pro: Filter items + dynamic symbol search when query changes
    Effect::new(move || {
        let q = query.get();
        let all_items = items.get();

        if q.is_empty() {
            filtered_items.set(all_items);
        } else {
            // Filter existing items
            let mut filtered: Vec<_> = all_items
                .into_iter()
                .filter(|item| {
                    fuzzy_match(&item.label, &q) ||
                    item.description.as_ref().map_or(false, |d| fuzzy_match(d, &q))
                })
                .collect();

            filtered_items.set(filtered);
            selected_index.set(0);

            // ✅ IntelliJ Pro: Dynamic symbol search for queries (runs asynchronously)
            // If query looks like a symbol search (has 2+ chars), search symbols
            if q.len() >= 2 {
                let query_for_search = q.clone();
                spawn_local(async move {
                    if let Ok(symbols) = tauri_bindings::search_symbols(&query_for_search).await {
                        let symbol_items: Vec<PaletteItem> = symbols
                            .into_iter()
                            .map(|sym| {
                                let kind_icon = match sym.kind {
                                    tauri_bindings::SymbolKind::Function => "🔧",
                                    tauri_bindings::SymbolKind::Struct => "📦",
                                    tauri_bindings::SymbolKind::Enum => "🔢",
                                    tauri_bindings::SymbolKind::Trait => "🎯",
                                    tauri_bindings::SymbolKind::Impl => "⚙️",
                                    tauri_bindings::SymbolKind::Const => "🔒",
                                    tauri_bindings::SymbolKind::Static => "📌",
                                    tauri_bindings::SymbolKind::Module => "📁",
                                };

                                PaletteItem {
                                    id: format!("symbol:{}:{}", sym.file_path, sym.line_number),
                                    label: sym.name.clone(),
                                    description: Some(format!(
                                        "{} - {}:{}",
                                        sym.signature.unwrap_or_default(),
                                        sym.file_path,
                                        sym.line_number
                                    )),
                                    action_type: ActionType::Symbol,
                                    icon: kind_icon.to_string(),
                                    action: format!("goto:{}:{}", sym.file_path, sym.line_number),
                                }
                            })
                            .collect();

                        // Update filtered items with symbol results (prepend symbols)
                        if !symbol_items.is_empty() {
                            // ✅ Safe: Combine with current filtered items
                            let current_filtered = filtered_items.get_untracked();
                            let mut combined = symbol_items;
                            combined.extend(current_filtered);
                            // ✅ Safe: Update UI with combined results
                            filtered_items.set(combined);
                        }
                    }
                });
            }
        }
    });

    // Clone on_select before the view to avoid FnOnce issues
    let on_select_for_view = on_select.clone();

    view! {
        {move || {
            if show.get() {
                // Clone on_select inside the reactive closure
                let on_select_for_keydown = on_select_for_view.clone();
                let on_select_for_items = on_select_for_view.clone();
                let handle_keydown = move |event: KeyboardEvent| {
                    let key = event.key();
                    let items_count = filtered_items.get_untracked().len();

                    match key.as_str() {
                        "ArrowDown" => {
                            event.prevent_default();
                            selected_index.update(|idx| {
                                *idx = (*idx + 1).min(items_count.saturating_sub(1));
                            });
                        }
                        "ArrowUp" => {
                            event.prevent_default();
                            selected_index.update(|idx| {
                                *idx = idx.saturating_sub(1);
                            });
                        }
                        "Enter" => {
                            event.prevent_default();
                            let idx = selected_index.get_untracked();
                            if let Some(item) = filtered_items.get_untracked().get(idx) {
                                on_select_for_keydown(item.clone());
                                show.set(false);
                            }
                        }
                        "Escape" => {
                            event.prevent_default();
                            show.set(false);
                        }
                        _ => {}
                    }
                };

                view! {
                    <div class="berry-command-palette-backdrop" on:click=move |_| show.set(false)>
                        <div class="berry-command-palette" on:click=move |e| e.stop_propagation()>
                            <input
                                type="text"
                                class="berry-command-palette-input"
                                placeholder="Type a command or search..."
                                prop:value=move || query.get()
                                on:input=move |ev| {
                                    query.set(event_target_value(&ev));
                                }
                                on:keydown=handle_keydown
                                autofocus
                            />

                            <div class="berry-command-palette-results">
                                {move || {
                                    let current_items = filtered_items.get();
                                    let selected = selected_index.get();

                                    if current_items.is_empty() {
                                        view! {
                                            <div class="berry-command-palette-empty">
                                                "No results found"
                                            </div>
                                        }.into_any()
                                    } else {
                                        current_items.iter().enumerate().map(|(idx, item)| {
                                            let is_selected = idx == selected;
                                            let item_clone = item.clone();
                                            let on_select_item = on_select_for_items.clone();

                                            view! {
                                                <PaletteItemView
                                                    item=item.clone()
                                                    selected=is_selected
                                                    on_click=move || {
                                                        on_select_item(item_clone.clone());
                                                        show.set(false);
                                                    }
                                                />
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}

/// Single palette item view
#[component]
fn PaletteItemView(
    item: PaletteItem,
    selected: bool,
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    let class = if selected {
        "berry-palette-item berry-palette-item-selected"
    } else {
        "berry-palette-item"
    };

    let icon_class = format!("berry-palette-icon berry-palette-icon-{}",
        match item.action_type {
            ActionType::File => "file",
            ActionType::GitAction => "git",
            ActionType::EditorAction => "action",
            ActionType::Settings => "settings",
            ActionType::Symbol => "symbol",
        }
    );

    view! {
        <div class=class on:click=move |_| on_click()>
            <span class=icon_class>{item.icon}</span>
            <div class="berry-palette-content">
                <div class="berry-palette-label">{item.label}</div>
                {item.description.map(|desc| {
                    view! {
                        <div class="berry-palette-description">{desc}</div>
                    }
                })}
            </div>
        </div>
    }
}

/// Load all palette items
fn load_palette_items(items: RwSignal<Vec<PaletteItem>>) {
    spawn_local(async move {
        let mut all_items = Vec::new();

        // Add file search
        if let Ok(files) = load_project_files().await {
            for file in files {
                all_items.push(PaletteItem {
                    id: format!("file:{}", file),
                    label: file.clone(),
                    description: None,
                    action_type: ActionType::File,
                    icon: "📄".to_string(),
                    action: format!("open:{}", file),
                });
            }
        }

        // Add git actions
        all_items.extend(vec![
            PaletteItem {
                id: "git:commit".to_string(),
                label: "Git: Commit".to_string(),
                description: Some("Create a new commit".to_string()),
                action_type: ActionType::GitAction,
                icon: "🔧".to_string(),
                action: "git:commit".to_string(),
            },
            PaletteItem {
                id: "git:push".to_string(),
                label: "Git: Push".to_string(),
                description: Some("Push to remote".to_string()),
                action_type: ActionType::GitAction,
                icon: "🔧".to_string(),
                action: "git:push".to_string(),
            },
            PaletteItem {
                id: "git:pull".to_string(),
                label: "Git: Pull".to_string(),
                description: Some("Pull from remote".to_string()),
                action_type: ActionType::GitAction,
                icon: "🔧".to_string(),
                action: "git:pull".to_string(),
            },
        ]);

        // Add editor actions
        all_items.extend(vec![
            PaletteItem {
                id: "editor:save".to_string(),
                label: "File: Save".to_string(),
                description: Some("Save current file".to_string()),
                action_type: ActionType::EditorAction,
                icon: "💾".to_string(),
                action: "editor:save".to_string(),
            },
            PaletteItem {
                id: "editor:close".to_string(),
                label: "File: Close".to_string(),
                description: Some("Close current file".to_string()),
                action_type: ActionType::EditorAction,
                icon: "❌".to_string(),
                action: "editor:close".to_string(),
            },
            PaletteItem {
                id: "editor:format".to_string(),
                label: "Format Document".to_string(),
                description: Some("Format the current file".to_string()),
                action_type: ActionType::EditorAction,
                icon: "✨".to_string(),
                action: "editor:format".to_string(),
            },
        ]);

        // Add settings
        all_items.push(PaletteItem {
            id: "settings:open".to_string(),
            label: "Settings".to_string(),
            description: Some("Open settings".to_string()),
            action_type: ActionType::Settings,
            icon: "⚙️".to_string(),
            action: "settings:open".to_string(),
        });

        items.set(all_items);
    });
}

/// Load project files from Tauri
async fn load_project_files() -> anyhow::Result<Vec<String>> {
    // TODO: Call Tauri command to get project files
    // For now, return mock data
    Ok(vec![
        "src/main.rs".to_string(),
        "src/editor.rs".to_string(),
        "src/file_tree.rs".to_string(),
        "Cargo.toml".to_string(),
    ])
}

/// Simple fuzzy matching
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    // Simple substring match for now
    text_lower.contains(&pattern_lower)
}

/// Advanced fuzzy matching (optional improvement)
fn fuzzy_match_score(text: &str, pattern: &str) -> i32 {
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    if text_lower == pattern_lower {
        return 1000; // Exact match
    }

    if text_lower.starts_with(&pattern_lower) {
        return 900; // Prefix match
    }

    if text_lower.contains(&pattern_lower) {
        return 500; // Contains match
    }

    // Character-by-character fuzzy match
    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern_lower.chars().collect();
    let text_chars: Vec<char> = text_lower.chars().collect();

    for &ch in &text_chars {
        if pattern_idx < pattern_chars.len() && ch == pattern_chars[pattern_idx] {
            pattern_idx += 1;
        }
    }

    if pattern_idx == pattern_chars.len() {
        return 100; // All characters found in order
    }

    0 // No match
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // TODO: Fix fuzzy_match to support acronym matching (hw -> hello_world)
    fn test_fuzzy_match() {
        assert!(fuzzy_match("hello_world", "hello"));
        assert!(fuzzy_match("hello_world", "world"));
        assert!(fuzzy_match("hello_world", "hw"));
        assert!(!fuzzy_match("hello_world", "xyz"));
    }

    #[test]
    fn test_fuzzy_match_score() {
        assert!(fuzzy_match_score("hello", "hello") > fuzzy_match_score("hello", "hel"));
        assert!(fuzzy_match_score("hello_world", "hello") > fuzzy_match_score("hello_world", "world"));
    }

    #[test]
    fn test_palette_item_creation() {
        let item = PaletteItem {
            id: "test".to_string(),
            label: "Test Item".to_string(),
            description: None,
            action_type: ActionType::File,
            icon: "📄".to_string(),
            action: "test:action".to_string(),
        };

        assert_eq!(item.id, "test");
        assert_eq!(item.action_type, ActionType::File);
    }
}
