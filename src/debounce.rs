//! Debouncing Utilities
//!
//! Provides debouncing for input events to reduce LSP calls and improve performance.

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Handle to a timeout that can be canceled
#[derive(Clone, Copy)]
pub struct TimeoutHandle {
    id: i32,
}

impl TimeoutHandle {
    /// Cancel the timeout
    pub fn cancel(&self) {
        let window = web_sys::window().expect("no global window");
        window.clear_timeout_with_handle(self.id);
    }
}

/// Debouncer for function calls
pub struct Debouncer {
    /// Timeout duration in milliseconds
    timeout_ms: i32,
    /// Pending timeout handle
    pending: Rc<Cell<Option<TimeoutHandle>>>,
}

impl Debouncer {
    /// Create a new debouncer with specified timeout
    pub fn new(timeout_ms: i32) -> Self {
        Self {
            timeout_ms,
            pending: Rc::new(Cell::new(None)),
        }
    }

    /// Debounce a function call
    pub fn debounce<F>(&self, callback: F)
    where
        F: FnOnce() + 'static,
    {
        // Cancel any pending timeout
        if let Some(handle) = self.pending.take() {
            handle.cancel();
        }

        // Create new timeout
        let window = web_sys::window().expect("no global window");
        let closure = Closure::once(callback);

        let timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                self.timeout_ms,
            )
            .expect("failed to set timeout");

        closure.forget();

        // Store the handle
        self.pending.set(Some(TimeoutHandle { id: timeout_id }));
    }

    /// Cancel any pending debounced call
    pub fn cancel(&self) {
        if let Some(handle) = self.pending.take() {
            handle.cancel();
        }
    }
}

impl Drop for Debouncer {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_debouncer_creation() {
        let debouncer = Debouncer::new(300);
        assert_eq!(debouncer.timeout_ms, 300);
    }

    #[wasm_bindgen_test]
    fn test_debouncer_cancel() {
        let debouncer = Debouncer::new(300);

        // Schedule a call
        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();
        debouncer.debounce(move || {
            called_clone.set(true);
        });

        // Cancel it immediately
        debouncer.cancel();

        // Verify no timeout is pending
        assert!(debouncer.pending.get().is_none());
    }
}
