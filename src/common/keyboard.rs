//! Keyboard Shortcut System
//! Global keyboard shortcut management

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{KeyboardEvent, window};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on Mac, Win key on Windows
}

impl KeyBinding {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }

    pub fn matches(&self, event: &KeyboardEvent) -> bool {
        self.key.to_lowercase() == event.key().to_lowercase()
            && self.ctrl == event.ctrl_key()
            && self.shift == event.shift_key()
            && self.alt == event.alt_key()
            && self.meta == event.meta_key()
    }

    /// Cross-platform shortcut (Cmd on Mac, Ctrl on other platforms)
    pub fn platform_modifier(key: &str) -> Self {
        #[cfg(target_os = "macos")]
        return Self::new(key).with_meta();

        #[cfg(not(target_os = "macos"))]
        return Self::new(key).with_ctrl();
    }
}

pub type ShortcutCallback = Box<dyn Fn()>;

pub struct KeyboardShortcuts {
    bindings: HashMap<String, (KeyBinding, ShortcutCallback)>,
}

impl KeyboardShortcuts {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: &str, binding: KeyBinding, callback: impl Fn() + 'static) {
        self.bindings.insert(id.to_string(), (binding, Box::new(callback)));
    }

    pub fn unregister(&mut self, id: &str) {
        self.bindings.remove(id);
    }

    pub fn count(&self) -> usize {
        self.bindings.len()
    }

    pub fn handle_event(&self, event: &KeyboardEvent) -> bool {
        for (_id, (binding, callback)) in self.bindings.iter() {
            if binding.matches(event) {
                callback();
                return true; // Shortcut was handled
            }
        }
        false
    }
}

/// Global keyboard shortcut hook
pub fn use_keyboard_shortcuts<F>(setup: F)
where
    F: Fn(&mut KeyboardShortcuts) + 'static,
{
    let shortcuts = std::rc::Rc::new(std::cell::RefCell::new(KeyboardShortcuts::new()));
    setup(&mut shortcuts.borrow_mut());

    Effect::new(move |_| {
        if let Some(win) = window() {
            let shortcuts_clone = shortcuts.clone();

            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::Event| {
                if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
                    if shortcuts_clone.borrow().handle_event(&keyboard_event) {
                        keyboard_event.prevent_default();
                        keyboard_event.stop_propagation();
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let _ = win.add_event_listener_with_callback(
                "keydown",
                closure.as_ref().unchecked_ref(),
            );

            closure.forget(); // Keep the closure alive
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keybinding_new() {
        let binding = KeyBinding::new("w");
        assert_eq!(binding.key, "w");
        assert!(!binding.ctrl);
        assert!(!binding.shift);
        assert!(!binding.alt);
        assert!(!binding.meta);
    }

    #[test]
    fn test_keybinding_with_modifiers() {
        let binding = KeyBinding::new("s")
            .with_ctrl()
            .with_shift();

        assert_eq!(binding.key, "s");
        assert!(binding.ctrl);
        assert!(binding.shift);
        assert!(!binding.alt);
        assert!(!binding.meta);
    }

    #[test]
    fn test_keyboard_shortcuts_register() {
        let mut shortcuts = KeyboardShortcuts::new();
        let called = std::rc::Rc::new(std::cell::RefCell::new(false));
        let called_clone = called.clone();

        shortcuts.register(
            "test",
            KeyBinding::new("t").with_ctrl(),
            move || {
                *called_clone.borrow_mut() = true;
            },
        );

        assert_eq!(shortcuts.bindings.len(), 1);
    }

    #[test]
    fn test_keyboard_shortcuts_unregister() {
        let mut shortcuts = KeyboardShortcuts::new();
        shortcuts.register(
            "test",
            KeyBinding::new("t"),
            || {},
        );

        shortcuts.unregister("test");
        assert_eq!(shortcuts.bindings.len(), 0);
    }

    #[test]
    fn test_keybinding_equality() {
        let binding1 = KeyBinding::new("a").with_ctrl();
        let binding2 = KeyBinding::new("a").with_ctrl();
        let binding3 = KeyBinding::new("b").with_ctrl();

        assert_eq!(binding1, binding2);
        assert_ne!(binding1, binding3);
    }
}
