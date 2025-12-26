//! Refactor Menu Component
//! Context menu for refactoring operations

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use super::{RefactorOperation, Position, Range};

#[derive(Debug, Clone)]
pub struct RefactorContext {
    pub file_path: String,
    pub position: Position,
    pub selection: Option<Range>,
}

#[component]
pub fn RefactorMenu(
    context: RefactorContext,
    on_select: impl Fn(RefactorOperation) + Clone + 'static,
    on_close: impl Fn() + 'static,
) -> impl IntoView {
    let available_operations = vec![
        RefactorOperation::Rename,
        RefactorOperation::ExtractMethod,
        RefactorOperation::InlineVariable,
        RefactorOperation::OptimizeImports,
        RefactorOperation::MoveSymbol,
        RefactorOperation::ChangeSignature,
    ];

    view! {
        <div class="refactor-menu" style="position: absolute; background: #252526; border: 1px solid #454545; border-radius: 4px; padding: 4px 0; min-width: 200px; box-shadow: 0 2px 8px rgba(0,0,0,0.3); z-index: 1000;">
            <div style="padding: 8px 12px; color: #858585; font-size: 11px; font-weight: bold; border-bottom: 1px solid #454545;">
                "REFACTOR"
            </div>
            {available_operations.into_iter().map(|op| {
                let op_clone = op;
                let on_select_clone = on_select.clone();
                view! {
                    <div
                        class="refactor-menu-item"
                        style="padding: 6px 12px; cursor: pointer; display: flex; justify-content: space-between; align-items: center; color: #cccccc; font-size: 13px;"
                        on:click=move |_| {
                            on_select_clone(op_clone);
                        }
                        on:mouseenter=move |e| {
                            if let Some(target) = e.target() {
                                let element = target.dyn_into::<web_sys::HtmlElement>().unwrap();
                                let _ = element.style().set_property("background", "#094771");
                            }
                        }
                        on:mouseleave=move |e| {
                            if let Some(target) = e.target() {
                                let element = target.dyn_into::<web_sys::HtmlElement>().unwrap();
                                let _ = element.style().set_property("background", "transparent");
                            }
                        }
                    >
                        <span>{op.label()}</span>
                        <span style="color: #858585; font-size: 11px; margin-left: 16px;">{op.shortcut()}</span>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn RefactorDialog(
    operation: RefactorOperation,
    context: RefactorContext,
    on_apply: impl Fn(RefactorParams) + 'static,
    on_cancel: impl Fn() + 'static,
) -> impl IntoView {
    let input_value = RwSignal::new(String::new());

    view! {
        <div class="refactor-dialog-overlay" style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;">
            <div class="refactor-dialog" style="background: #252526; border: 1px solid #454545; border-radius: 6px; min-width: 400px; max-width: 600px; padding: 16px; box-shadow: 0 4px 16px rgba(0,0,0,0.5);">
                <div style="font-size: 16px; font-weight: bold; color: #cccccc; margin-bottom: 16px;">
                    {operation.label()}
                </div>

                {match operation {
                    RefactorOperation::Rename => view! {
                        <div>
                            <label style="display: block; color: #cccccc; font-size: 13px; margin-bottom: 8px;">
                                "New name:"
                            </label>
                            <input
                                type="text"
                                prop:value=move || input_value.get()
                                on:input=move |e| {
                                    input_value.set(event_target_value(&e));
                                }
                                style="width: 100%; padding: 8px; background: #3c3c3c; border: 1px solid #454545; border-radius: 4px; color: #cccccc; font-size: 13px;"
                                placeholder="Enter new name"
                            />
                        </div>
                    }.into_any(),
                    RefactorOperation::ExtractMethod => view! {
                        <div>
                            <label style="display: block; color: #cccccc; font-size: 13px; margin-bottom: 8px;">
                                "Method name:"
                            </label>
                            <input
                                type="text"
                                prop:value=move || input_value.get()
                                on:input=move |e| {
                                    input_value.set(event_target_value(&e));
                                }
                                style="width: 100%; padding: 8px; background: #3c3c3c; border: 1px solid #454545; border-radius: 4px; color: #cccccc; font-size: 13px;"
                                placeholder="Enter method name"
                            />
                        </div>
                    }.into_any(),
                    RefactorOperation::ChangeSignature => view! {
                        <div>
                            <label style="display: block; color: #cccccc; font-size: 13px; margin-bottom: 8px;">
                                "New signature:"
                            </label>
                            <input
                                type="text"
                                prop:value=move || input_value.get()
                                on:input=move |e| {
                                    input_value.set(event_target_value(&e));
                                }
                                style="width: 100%; padding: 8px; background: #3c3c3c; border: 1px solid #454545; border-radius: 4px; color: #cccccc; font-size: 13px;"
                                placeholder="fn name(params)"
                            />
                        </div>
                    }.into_any(),
                    _ => view! {
                        <div style="color: #cccccc; font-size: 13px;">
                            "Apply " {operation.label()} "?"
                        </div>
                    }.into_any(),
                }}

                <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px;">
                    <button
                        on:click=move |_| on_cancel()
                        style="padding: 6px 16px; background: #3c3c3c; border: 1px solid #454545; border-radius: 4px; color: #cccccc; cursor: pointer; font-size: 13px;"
                    >
                        "Cancel"
                    </button>
                    <button
                        on:click=move |_| {
                            let params = match operation {
                                RefactorOperation::Rename => RefactorParams::Rename {
                                    new_name: input_value.get(),
                                },
                                RefactorOperation::ExtractMethod => RefactorParams::ExtractMethod {
                                    method_name: input_value.get(),
                                    range: context.selection.clone().unwrap_or(Range {
                                        start: context.position.clone(),
                                        end: context.position.clone(),
                                    }),
                                },
                                RefactorOperation::ChangeSignature => RefactorParams::ChangeSignature {
                                    new_signature: input_value.get(),
                                },
                                _ => RefactorParams::Simple,
                            };
                            on_apply(params);
                        }
                        style="padding: 6px 16px; background: #0e639c; border: 1px solid #0e639c; border-radius: 4px; color: #ffffff; cursor: pointer; font-size: 13px; font-weight: bold;"
                    >
                        "Apply"
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Debug, Clone)]
pub enum RefactorParams {
    Rename { new_name: String },
    ExtractMethod { method_name: String, range: Range },
    InlineVariable,
    OptimizeImports,
    MoveSymbol { target_file: String },
    ChangeSignature { new_signature: String },
    Simple,
}
