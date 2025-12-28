//! UI Components for BerryEditor
//! 100% Rust - No JavaScript!

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::file_tree::FileTreePanel;
use crate::editor::EditorPanel;
use crate::debugger::{DebugToolbar, VariablesPanel, CallStackPanel, WatchPanel, DebugConsole};
use crate::debugger::session::DebugSession;
use crate::diagnostics_panel::DiagnosticsPanel;
use crate::lsp_ui::{Diagnostic, LspIntegration};
use crate::common::splitter::{ResizableSplitter, Orientation};

#[component]
pub fn EditorApp() -> impl IntoView {

    // Debug session
    let debug_session = DebugSession::new();

    // LSP integration
    let diagnostics = RwSignal::new(Vec::<Diagnostic>::new());

    // File selection state (shared between FileTree and Editor)
    let selected_file = RwSignal::new(Option::<(String, String)>::None); // (path, content)

    // UI state
    let show_debug_panels = RwSignal::new(false);
    let selected_frame = RwSignal::new(None);


    view! {
        <div class="berry-editor-container" style="border: 5px solid red; background: #1e1e1e;">
            // Left Sidebar - File Tree
            <FileTreePanel on_file_select=selected_file />

            // Main Editor Area
            <div class="berry-editor-main-area" style="border: 5px solid blue;">
                // Debug Toolbar (shown when debugging)
                {move || {
                    if show_debug_panels.get() {
                        view! {
                            <DebugToolbar session=debug_session.clone() />
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}

                // Editor Panel
                <EditorPanel selected_file=selected_file />

                // Bottom Panel - Diagnostics
                <div class="berry-editor-bottom-panel">
                    <DiagnosticsPanel
                        diagnostics=diagnostics
                        on_click=move |line, character| {
                            // Jump to diagnostic location
                        }
                    />
                </div>
            </div>

            // Right Sidebar - Debug Panels (shown when debugging)
            {move || {
                if show_debug_panels.get() {
                    view! {
                        <div class="berry-editor-debug-sidebar">
                            <VariablesPanel scopes=debug_session.scopes />
                            <CallStackPanel
                                frames=debug_session.stack_frames
                                selected_frame=selected_frame
                                on_frame_click=move |frame_id| {
                                    selected_frame.set(Some(frame_id));
                                    // Load variables for this frame
                                    let session = debug_session.clone();
                                    spawn_local(async move {
                                        let _ = session.get_variables(frame_id).await;
                                    });
                                }
                            />
                            <WatchPanel
                                watches=RwSignal::new(Vec::new())
                                session=debug_session.clone()
                            />
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Status Bar
            <div class="berry-editor-status-bar">
                <div class="berry-editor-status-left">
                    <span>"BerryEditor"</span>
                    <span style="margin-left: 16px;">"100% Rust"</span>
                </div>
                <div class="berry-editor-status-right">
                    <button
                        class="berry-status-button"
                        on:click=move |_| {
                            show_debug_panels.update(|v| *v = !*v);
                        }
                    >
                        {move || if show_debug_panels.get() { "Hide Debug" } else { "Show Debug" }}
                    </button>
                    <span style="margin-left: 16px;">
                        {move || {
                            let diag_count = diagnostics.get().len();
                            if diag_count > 0 {
                                format!("{} problems", diag_count)
                            } else {
                                "No problems".to_string()
                            }
                        }}
                    </span>
                </div>
            </div>
        </div>
    }
}
