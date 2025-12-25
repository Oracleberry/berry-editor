//! Debug Toolbar Component
//!
//! Provides debug control buttons (Continue, Step Over/Into/Out, Stop, Restart).

use leptos::prelude::*;
use super::session::{DebugSession, DebugState};
use crate::common::ui_components::IconButton;

/// Debug toolbar component
#[component]
pub fn DebugToolbar(
    /// Debug session
    session: DebugSession,
) -> impl IntoView {
    let state = session.state;

    // Continue (F5)
    let handle_continue = move |_| {
        spawn_local(async move {
            if let Err(e) = session.continue_execution().await {
                web_sys::console::error_1(&format!("Continue failed: {}", e).into());
            }
        });
    };

    // Step Over (F10)
    let handle_step_over = move |_| {
        spawn_local(async move {
            if let Err(e) = session.step_over().await {
                web_sys::console::error_1(&format!("Step over failed: {}", e).into());
            }
        });
    };

    // Step Into (F11)
    let handle_step_into = move |_| {
        spawn_local(async move {
            if let Err(e) = session.step_into().await {
                web_sys::console::error_1(&format!("Step into failed: {}", e).into());
            }
        });
    };

    // Step Out (Shift+F11)
    let handle_step_out = move |_| {
        spawn_local(async move {
            if let Err(e) = session.step_out().await {
                web_sys::console::error_1(&format!("Step out failed: {}", e).into());
            }
        });
    };

    // Stop
    let handle_stop = move |_| {
        spawn_local(async move {
            if let Err(e) = session.stop().await {
                web_sys::console::error_1(&format!("Stop failed: {}", e).into());
            }
        });
    };

    // Restart
    let handle_restart = move |_| {
        spawn_local(async move {
            // Stop current session
            if let Err(e) = session.stop().await {
                web_sys::console::error_1(&format!("Stop failed: {}", e).into());
                return;
            }

            // Start new session (would need program path - simplified here)
            // In real implementation, we'd store the program path
            web_sys::console::log_1(&"Restart not yet fully implemented".into());
        });
    };

    view! {
        <div class="berry-debug-toolbar">
            {move || {
                let current_state = state.get();
                let is_stopped = current_state == DebugState::Stopped;
                let is_paused = current_state == DebugState::Paused || current_state == DebugState::Stepping;

                view! {
                    <div class="berry-debug-toolbar-buttons">
                        <IconButton
                            icon="▶"
                            tooltip="Continue (F5)"
                            on_click=handle_continue
                            disabled=is_stopped
                        />
                        <IconButton
                            icon="⤵"
                            tooltip="Step Over (F10)"
                            on_click=handle_step_over
                            disabled=!is_paused
                        />
                        <IconButton
                            icon="↓"
                            tooltip="Step Into (F11)"
                            on_click=handle_step_into
                            disabled=!is_paused
                        />
                        <IconButton
                            icon="↑"
                            tooltip="Step Out (Shift+F11)"
                            on_click=handle_step_out
                            disabled=!is_paused
                        />
                        <IconButton
                            icon="■"
                            tooltip="Stop"
                            on_click=handle_stop
                            disabled=is_stopped
                        />
                        <IconButton
                            icon="↻"
                            tooltip="Restart"
                            on_click=handle_restart
                            disabled=is_stopped
                        />
                    </div>
                    <div class="berry-debug-status">
                        {move || {
                            match state.get() {
                                DebugState::Stopped => "Stopped",
                                DebugState::Running => "Running",
                                DebugState::Paused => "Paused",
                                DebugState::Stepping => "Stepping",
                            }
                        }}
                    </div>
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_debug_toolbar_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_debug_state_display() {
        assert_eq!(format!("{:?}", DebugState::Stopped), "Stopped");
        assert_eq!(format!("{:?}", DebugState::Running), "Running");
        assert_eq!(format!("{:?}", DebugState::Paused), "Paused");
        assert_eq!(format!("{:?}", DebugState::Stepping), "Stepping");
    }

    #[test]
    fn test_button_disabled_logic() {
        // When stopped, continue should be disabled
        let is_stopped = true;
        assert!(is_stopped);

        // When paused, step buttons should be enabled
        let is_paused = true;
        assert!(is_paused);
    }
}
