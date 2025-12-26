//! Context Menu Component
//! Reusable right-click context menu

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub action: String,
    pub disabled: bool,
    pub separator_after: bool,
}

impl MenuItem {
    pub fn new(label: &str, action: &str) -> Self {
        Self {
            label: label.to_string(),
            action: action.to_string(),
            disabled: false,
            separator_after: false,
        }
    }

    pub fn with_separator(mut self) -> Self {
        self.separator_after = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[component]
pub fn ContextMenu(
    items: Vec<MenuItem>,
    position: RwSignal<Option<(i32, i32)>>,
    on_action: impl Fn(String) + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let is_visible = Memo::new(move |_| position.get().is_some());

    // Close on click outside
    Effect::new(move |_| {
        if is_visible.get() {
            let position_clone = position.clone();
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                position_clone.set(None);
            }) as Box<dyn FnMut(_)>);

            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                let _ = document.add_event_listener_with_callback(
                    "click",
                    closure.as_ref().unchecked_ref(),
                );
                closure.forget();
            }
        }
    });

    view! {
        {move || {
            if let Some((x, y)) = position.get() {
                view! {
                    <div
                        class="berry-context-menu"
                        style=format!("left: {}px; top: {}px;", x, y)
                    >
                        {items.iter().map(|item| {
                            let action = item.action.clone();
                            let on_action_clone = on_action.clone();
                            let disabled = item.disabled;
                            let separator_after = item.separator_after;

                            view! {
                                <>
                                    <div
                                        class=if disabled { "berry-context-menu-item disabled" } else { "berry-context-menu-item" }
                                        on:click=move |e: MouseEvent| {
                                            if !disabled {
                                                e.stop_propagation();
                                                on_action_clone(action.clone());
                                                position.set(None);
                                            }
                                        }
                                    >
                                        {item.label.clone()}
                                    </div>
                                    {if separator_after {
                                        view! { <div class="berry-context-menu-separator"></div> }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }}
                                </>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}
