use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::html::Input;
use serde::{Deserialize, Serialize};
use crate::tauri_bindings_berrycode::*;

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[component]
pub fn BerryCodePanel(
    /// Project root path
    #[prop(into)]
    project_path: Signal<String>,
) -> impl IntoView {
    leptos::logging::log!("🚀 BerryCodePanel component created!");

    let user_input = RwSignal::new(String::new());
    let chat_messages = RwSignal::new(Vec::<ChatMessage>::new());
    let is_loading = RwSignal::new(false);
    let context_files = RwSignal::new(Vec::<String>::new());

    // Input element reference for focus
    let input_ref = NodeRef::<Input>::new();

    // Initialize BerryCode session on mount
    create_effect(move |_| {
        let path = project_path.get();
        leptos::logging::log!("🔧 Initializing BerryCode session for: {}", path);
        spawn_local(async move {
            match berrycode_init(
                Some("gpt-4o".to_string()),
                Some("code".to_string()),
                Some(path.clone())
            ).await {
                Ok(msg) => {
                    leptos::logging::log!("✅ BerryCode initialized: {}", msg);
                }
                Err(e) => {
                    leptos::logging::error!("❌ BerryCode init failed: {}", e);
                }
            }
        });
    });

    // Send message logic
    let do_send_message = move || {
        let message = user_input.get();
        if message.trim().is_empty() {
            return;
        }

        // Add user message to chat
        chat_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "user".to_string(),
                content: message.clone(),
            });
        });

        user_input.set(String::new());
        is_loading.set(true);

        spawn_local(async move {
            match berrycode_chat(message).await {
                Ok(response) => {
                    chat_messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response,
                        });
                    });
                }
                Err(e) => {
                    leptos::logging::error!("❌ Chat error: {}", e);
                    chat_messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: format!("Error: {}", e),
                        });
                    });
                }
            }
            is_loading.set(false);
        });
    };

    // Button click handler
    let send_message_click = move |_ev: leptos::ev::MouseEvent| {
        do_send_message();
    };

    // Enter key handler
    let send_message_enter = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send_message();
        }
    };

    // Add current file to context
    let add_current_file = move |_| {
        let path = project_path.get();
        spawn_local(async move {
            match berrycode_add_file(path.clone()).await {
                Ok(msg) => {
                    leptos::logging::log!("✅ {}", msg);
                    context_files.update(|files| {
                        if !files.contains(&path) {
                            files.push(path);
                        }
                    });
                }
                Err(e) => {
                    leptos::logging::error!("❌ Add file error: {}", e);
                }
            }
        });
    };

    view! {
        <div class="berrycode-panel" style="display: flex; flex-direction: column; height: 100%; background: #1E1F22; color: #BCBEC4; font-family: 'JetBrains Mono', monospace;">
            // Header
            <div style="padding: 12px; border-bottom: 1px solid #323232; font-weight: bold; font-size: 14px;">
                "🍓 BerryCode AI Assistant"
            </div>

            // Context files section
            <div style="padding: 8px; border-bottom: 1px solid #323232; background: #2B2D30; max-height: 100px; overflow-y: auto;">
                <div style="font-size: 11px; color: #808080; margin-bottom: 4px;">
                    "Context Files:"
                </div>
                <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                    {move || {
                        let files = context_files.get();
                        if files.is_empty() {
                            view! {
                                <span style="color: #808080; font-size: 11px;">"No files in context"</span>
                            }.into_any()
                        } else {
                            files.into_iter().map(|file| {
                                view! {
                                    <span style="background: #3C3F41; padding: 2px 6px; border-radius: 3px; font-size: 11px;">
                                        {file}
                                    </span>
                                }
                            }).collect_view().into_any()
                        }
                    }}
                    <button
                        on:click=add_current_file
                        style="background: #365880; color: white; border: none; padding: 2px 8px; border-radius: 3px; cursor: pointer; font-size: 11px;"
                    >
                        "+ Add Current File"
                    </button>
                </div>
            </div>

            // Chat messages area
            <div style="flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 12px;">
                {move || {
                    chat_messages.get().into_iter().map(|msg| {
                        let bg_color = if msg.role == "user" { "#2B2D30" } else { "#1E1F22" };
                        let border_color = if msg.role == "user" { "#365880" } else { "#4A7A3C" };

                        view! {
                            <div style=format!("background: {}; border-left: 3px solid {}; padding: 8px; border-radius: 4px;", bg_color, border_color)>
                                <div style="font-size: 10px; color: #808080; margin-bottom: 4px;">
                                    {if msg.role == "user" { "You" } else { "BerryCode" }}
                                </div>
                                <div style="font-size: 12px; white-space: pre-wrap;">
                                    {msg.content}
                                </div>
                            </div>
                        }
                    }).collect_view()
                }}
                {move || if is_loading.get() {
                    view! {
                        <div style="background: #1E1F22; border-left: 3px solid #4A7A3C; padding: 8px; border-radius: 4px;">
                            <div style="font-size: 10px; color: #808080; margin-bottom: 4px;">
                                "BerryCode"
                            </div>
                            <div style="font-size: 12px; color: #808080;">
                                "Thinking..."
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>

            // Input area
            <div style="padding: 12px; border-top: 1px solid #323232; display: flex; gap: 8px; background: #2B2D30;">
                <input
                    type="text"
                    node_ref=input_ref
                    placeholder="Ask BerryCode anything..."
                    style="flex: 1; background: #3C3F41; border: 1px solid #555555; color: #BCBEC4; padding: 8px; border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 12px;"
                    prop:value=move || user_input.get()
                    on:input=move |ev| {
                        user_input.set(event_target_value(&ev));
                    }
                    on:keydown=send_message_enter
                />
                <button
                    on:click=send_message_click
                    prop:disabled=move || user_input.get().trim().is_empty() || is_loading.get()
                    style="background: #365880; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-family: 'JetBrains Mono', monospace; font-size: 12px; font-weight: bold;"
                >
                    "Send"
                </button>
            </div>
        </div>
    }
}
