//! Resizable Splitter Component
//!
//! IntelliJ-style draggable panels

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent, window};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Resizable Splitter Component
#[component]
pub fn ResizableSplitter(
    /// Orientation: Horizontal (left/right) or Vertical (top/bottom)
    orientation: Orientation,
    /// Initial size in pixels
    initial_size: f64,
    /// Minimum size in pixels
    #[prop(default = 100.0)]
    min_size: f64,
    /// Maximum size in pixels (0 = no limit)
    #[prop(default = 0.0)]
    max_size: f64,
    /// Primary panel content
    primary: Children,
    /// Secondary panel content
    secondary: Children,
    /// Storage key for persisting size
    #[prop(optional)]
    storage_key: Option<String>,
) -> impl IntoView {
    // Load size from storage or use initial
    let stored_size = storage_key.as_ref().and_then(|key| {
        window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|storage| storage.get_item(key).ok().flatten())
            .and_then(|s| s.parse::<f64>().ok())
    });

    let size = RwSignal::new(stored_size.unwrap_or(initial_size));
    let is_dragging = RwSignal::new(false);
    let drag_start_size = RwSignal::new(0.0);
    let drag_start_pos = RwSignal::new(0.0);

    // Save size to storage
    let save_size = move |new_size: f64| {
        if let Some(key) = &storage_key {
            if let Some(storage) = window()
                .and_then(|w| w.local_storage().ok().flatten())
            {
                let _ = storage.set_item(key, &new_size.to_string());
            }
        }
    };

    // Mouse down on handle
    let handle_mousedown = move |event: MouseEvent| {
        event.prevent_default();
        is_dragging.set(true);
        drag_start_size.set(size.get_untracked());

        match orientation {
            Orientation::Horizontal => {
                drag_start_pos.set(event.client_x() as f64);
            }
            Orientation::Vertical => {
                drag_start_pos.set(event.client_y() as f64);
            }
        }
    };

    // Global mouse move handler
    let handle_mousemove = move |event: MouseEvent| {
        if !is_dragging.get_untracked() {
            return;
        }

        let start_size = drag_start_size.get_untracked();
        let start_pos = drag_start_pos.get_untracked();

        let delta = match orientation {
            Orientation::Horizontal => event.client_x() as f64 - start_pos,
            Orientation::Vertical => event.client_y() as f64 - start_pos,
        };

        let mut new_size = start_size + delta;

        // Apply constraints
        new_size = new_size.max(min_size);
        if max_size > 0.0 {
            new_size = new_size.min(max_size);
        }

        size.set(new_size);
    };

    // Global mouse up handler
    let handle_mouseup = move |_event: MouseEvent| {
        if is_dragging.get_untracked() {
            is_dragging.set(false);
            save_size(size.get_untracked());
        }
    };

    let container_class = match orientation {
        Orientation::Horizontal => "berry-splitter-container berry-splitter-horizontal",
        Orientation::Vertical => "berry-splitter-container berry-splitter-vertical",
    };

    let handle_class = match orientation {
        Orientation::Horizontal => "berry-splitter-handle berry-splitter-handle-horizontal",
        Orientation::Vertical => "berry-splitter-handle berry-splitter-handle-vertical",
    };

    view! {
        <div
            class=container_class
            on:mousemove=handle_mousemove
            on:mouseup=handle_mouseup
        >
            <div
                class="berry-splitter-primary"
                style=move || {
                    let current_size = size.get();
                    match orientation {
                        Orientation::Horizontal => format!("width: {}px", current_size),
                        Orientation::Vertical => format!("height: {}px", current_size),
                    }
                }
            >
                {primary()}
            </div>

            <div
                class=handle_class
                on:mousedown=handle_mousedown
            />

            <div class="berry-splitter-secondary">
                {secondary()}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orientation_equality() {
        assert_eq!(Orientation::Horizontal, Orientation::Horizontal);
        assert_ne!(Orientation::Horizontal, Orientation::Vertical);
    }

    #[test]
    fn test_size_constraints() {
        let min: f64 = 100.0;
        let max: f64 = 500.0;
        let mut size: f64 = 250.0;

        // Test min constraint
        size = 50.0;
        size = size.max(min);
        assert_eq!(size, 100.0);

        // Test max constraint
        size = 600.0;
        size = size.min(max);
        assert_eq!(size, 500.0);
    }
}
