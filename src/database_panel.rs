use leptos::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

// Re-export backend types for frontend use
pub use crate::tauri_bindings_database::{DbConnection, DbType, ConnectionTestResult};

#[component]
pub fn DatabasePanel(is_active: Signal<bool>) -> impl IntoView {
    let connections = RwSignal::new(Vec::<DbConnection>::new());
    let selected_connection = RwSignal::new(None::<String>);
    let show_add_dialog = RwSignal::new(false);
    let show_edit_dialog = RwSignal::new(false);
    let edit_connection = RwSignal::new(None::<DbConnection>);

    // Form state for Add/Edit dialog
    let form_name = RwSignal::new(String::new());
    let form_db_type = RwSignal::new(DbType::PostgreSQL);
    let form_host = RwSignal::new(String::from("localhost"));
    let form_port = RwSignal::new(String::from("5432"));
    let form_database = RwSignal::new(String::new());
    let form_username = RwSignal::new(String::new());
    let form_password = RwSignal::new(String::new());
    let form_ssl = RwSignal::new(false);
    let test_result = RwSignal::new(None::<ConnectionTestResult>);
    let is_testing = RwSignal::new(false);

    // Load connections on mount
    Effect::new(move |_| {
        if is_active.get() {
            leptos::task::spawn_local(async move {
                match crate::tauri_bindings_database::db_list_connections().await {
                    Ok(conns) => connections.set(conns),
                    Err(e) => leptos::logging::error!("Failed to load connections: {}", e),
                }
            });
        }
    });

    // Reload connections helper
    let reload_connections = move || {
        leptos::task::spawn_local(async move {
            match crate::tauri_bindings_database::db_list_connections().await {
                Ok(conns) => connections.set(conns),
                Err(e) => leptos::logging::error!("Failed to reload connections: {}", e),
            }
        });
    };

    view! {
        <div class="berry-editor-sidebar" style="background: #252526;">
            <div class="berry-editor-sidebar-header" style="
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding: 8px 12px;
                background: #2D2D30;
                border-bottom: 1px solid #1e1e1e;
                font-size: 12px;
                font-weight: 600;
                color: #cccccc;
            ">
                <span>"DATABASE TOOLS"</span>
                <button
                    on:click=move |_| show_add_dialog.set(true)
                    style="
                        background: none;
                        border: none;
                        color: #858585;
                        cursor: pointer;
                        font-size: 16px;
                        padding: 2px;
                    "
                    title="Add new database connection"
                >
                    <i class="codicon codicon-add"></i>
                </button>
            </div>

            <div class="db-connection-list" style="
                flex: 1;
                overflow-y: auto;
                padding: 8px;
            ">
                {move || {
                    let conns = connections.get();
                    if conns.is_empty() {
                        view! {
                            <div class="db-empty-state" style="
                                display: flex;
                                flex-direction: column;
                                align-items: center;
                                justify-content: center;
                                padding: 40px 20px;
                                color: #858585;
                                text-align: center;
                                gap: 16px;
                            ">
                                <i class="codicon codicon-database" style="font-size: 48px;"></i>
                                <p>"No database connections"</p>
                                <button
                                    on:click=move |_| show_add_dialog.set(true)
                                    style="
                                        background: #0e639c;
                                        border: none;
                                        color: white;
                                        padding: 8px 16px;
                                        cursor: pointer;
                                        font-size: 12px;
                                        border-radius: 4px;
                                    "
                                >
                                    "Add Connection"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        conns.iter().map(|conn| {
                            // Create separate clones for each closure to avoid ownership conflicts
                            let conn_id_for_select = conn.id.clone();
                            let conn_id_for_click = conn.id.clone();
                            let conn_id_for_delete = conn.id.clone();
                            let conn_name = conn.name.clone();
                            let conn_clone_for_test = conn.clone();
                            let conn_clone_for_edit = conn.clone();

                            let is_selected = Signal::derive(move || {
                                selected_connection.get().as_ref() == Some(&conn_id_for_select)
                            });

                            view! {
                                <div
                                    class="db-connection-item"
                                    class:selected=is_selected
                                    style=move || format!(
                                        "display: flex; align-items: center; padding: 6px 12px; cursor: pointer; \
                                         border-radius: 4px; gap: 8px; color: {}; background: {}; transition: background 0.15s;",
                                        if is_selected.get() { "#ffffff" } else { "#bbbbbb" },
                                        if is_selected.get() { "#2D2D30" } else { "transparent" }
                                    )
                                    on:click=move |_| selected_connection.set(Some(conn_id_for_click.clone()))
                                >
                                    <i class="codicon codicon-database" style="font-size: 16px;"></i>
                                    <span style="flex: 1; font-size: 12px;">{conn_name.clone()}</span>
                                    <div class="db-connection-actions" style="display: flex; gap: 4px;">
                                        <button
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                let conn_test = conn_clone_for_test.clone();
                                                leptos::task::spawn_local(async move {
                                                    match crate::tauri_bindings_database::db_test_connection(conn_test).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                leptos::logging::log!("✅ Connection successful: {}", result.message);
                                                            } else {
                                                                leptos::logging::error!("❌ Connection failed: {}", result.message);
                                                            }
                                                        }
                                                        Err(e) => leptos::logging::error!("Test connection error: {}", e),
                                                    }
                                                });
                                            }
                                            style="background: none; border: none; color: #858585; cursor: pointer; padding: 2px; font-size: 14px;"
                                            title="Test Connection"
                                        >
                                            <i class="codicon codicon-debug-start"></i>
                                        </button>
                                        <button
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                edit_connection.set(Some(conn_clone_for_edit.clone()));
                                                show_edit_dialog.set(true);
                                            }
                                            style="background: none; border: none; color: #858585; cursor: pointer; padding: 2px; font-size: 14px;"
                                            title="Edit"
                                        >
                                            <i class="codicon codicon-edit"></i>
                                        </button>
                                        <button
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                let id = conn_id_for_delete.clone();
                                                leptos::task::spawn_local(async move {
                                                    match crate::tauri_bindings_database::db_delete_connection(id).await {
                                                        Ok(_) => {
                                                            leptos::logging::log!("✅ Connection deleted");
                                                            reload_connections();
                                                        }
                                                        Err(e) => leptos::logging::error!("Delete error: {}", e),
                                                    }
                                                });
                                            }
                                            style="background: none; border: none; color: #858585; cursor: pointer; padding: 2px; font-size: 14px;"
                                            title="Delete"
                                        >
                                            <i class="codicon codicon-trash"></i>
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            // Add/Edit Dialog (Modal)
            {move || {
                if show_add_dialog.get() || show_edit_dialog.get() {
                    // Initialize form when opening edit dialog
                    if show_edit_dialog.get() {
                        if let Some(conn) = edit_connection.get() {
                            form_name.set(conn.name.clone());
                            form_db_type.set(conn.db_type.clone());
                            form_host.set(conn.host.clone().unwrap_or_default());
                            form_port.set(conn.port.map(|p| p.to_string()).unwrap_or_default());
                            form_database.set(conn.database.clone());
                            form_username.set(conn.username.clone().unwrap_or_default());
                            form_password.set(conn.password.clone().unwrap_or_default());
                            form_ssl.set(conn.ssl);
                        }
                    } else {
                        // Reset form for new connection
                        form_name.set(String::new());
                        form_db_type.set(DbType::PostgreSQL);
                        form_host.set(String::from("localhost"));
                        form_port.set(String::from("5432"));
                        form_database.set(String::new());
                        form_username.set(String::new());
                        form_password.set(String::new());
                        form_ssl.set(false);
                        test_result.set(None);
                    }

                    let is_sqlite = move || matches!(form_db_type.get(), DbType::SQLite);

                    view! {
                        <div style="
                            position: fixed;
                            top: 0;
                            left: 0;
                            right: 0;
                            bottom: 0;
                            background: rgba(0, 0, 0, 0.6);
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            z-index: 1000;
                        ">
                            <div style="
                                background: #2D2D30;
                                border: 1px solid #3e3e3e;
                                border-radius: 6px;
                                min-width: 500px;
                                max-width: 600px;
                                box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
                            ">
                                <div style="
                                    padding: 12px 16px;
                                    border-bottom: 1px solid #3e3e3e;
                                    font-size: 14px;
                                    font-weight: 600;
                                    color: #ffffff;
                                ">
                                    {if show_add_dialog.get() { "Add Database Connection" } else { "Edit Database Connection" }}
                                </div>

                                <div style="padding: 20px; color: #bbbbbb; display: flex; flex-direction: column; gap: 16px;">
                                    // Connection Name
                                    <div>
                                        <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                            "Connection Name"
                                        </label>
                                        <input
                                            type="text"
                                            prop:value=move || form_name.get()
                                            on:input=move |ev| form_name.set(event_target_value(&ev))
                                            placeholder="My Database"
                                            style="
                                                width: 100%;
                                                padding: 6px 8px;
                                                background: #1e1e1e;
                                                border: 1px solid #3e3e3e;
                                                border-radius: 4px;
                                                color: #cccccc;
                                                font-size: 12px;
                                            "
                                        />
                                    </div>

                                    // Database Type
                                    <div>
                                        <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                            "Database Type"
                                        </label>
                                        <select
                                            on:change=move |ev| {
                                                let value = event_target_value(&ev);
                                                form_db_type.set(match value.as_str() {
                                                    "MySQL" => DbType::MySQL,
                                                    "SQLite" => DbType::SQLite,
                                                    "MongoDB" => DbType::MongoDB,
                                                    _ => DbType::PostgreSQL,
                                                });
                                                // Update default port
                                                form_port.set(match value.as_str() {
                                                    "MySQL" => "3306",
                                                    "MongoDB" => "27017",
                                                    _ => "5432",
                                                }.to_string());
                                            }
                                            style="
                                                width: 100%;
                                                padding: 6px 8px;
                                                background: #1e1e1e;
                                                border: 1px solid #3e3e3e;
                                                border-radius: 4px;
                                                color: #cccccc;
                                                font-size: 12px;
                                            "
                                        >
                                            <option value="PostgreSQL" selected=move || matches!(form_db_type.get(), DbType::PostgreSQL)>"PostgreSQL"</option>
                                            <option value="MySQL" selected=move || matches!(form_db_type.get(), DbType::MySQL)>"MySQL"</option>
                                            <option value="SQLite" selected=move || matches!(form_db_type.get(), DbType::SQLite)>"SQLite"</option>
                                            <option value="MongoDB" selected=move || matches!(form_db_type.get(), DbType::MongoDB)>"MongoDB"</option>
                                        </select>
                                    </div>

                                    // Host (not for SQLite)
                                    {move || {
                                        if !is_sqlite() {
                                            view! {
                                                <div>
                                                    <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                                        "Host"
                                                    </label>
                                                    <input
                                                        type="text"
                                                        prop:value=move || form_host.get()
                                                        on:input=move |ev| form_host.set(event_target_value(&ev))
                                                        placeholder="localhost"
                                                        style="
                                                            width: 100%;
                                                            padding: 6px 8px;
                                                            background: #1e1e1e;
                                                            border: 1px solid #3e3e3e;
                                                            border-radius: 4px;
                                                            color: #cccccc;
                                                            font-size: 12px;
                                                        "
                                                    />
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    // Port (not for SQLite)
                                    {move || {
                                        if !is_sqlite() {
                                            view! {
                                                <div>
                                                    <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                                        "Port"
                                                    </label>
                                                    <input
                                                        type="text"
                                                        prop:value=move || form_port.get()
                                                        on:input=move |ev| form_port.set(event_target_value(&ev))
                                                        placeholder="5432"
                                                        style="
                                                            width: 100%;
                                                            padding: 6px 8px;
                                                            background: #1e1e1e;
                                                            border: 1px solid #3e3e3e;
                                                            border-radius: 4px;
                                                            color: #cccccc;
                                                            font-size: 12px;
                                                        "
                                                    />
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    // Database / File Path
                                    <div>
                                        <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                            {move || if is_sqlite() { "File Path" } else { "Database Name" }}
                                        </label>
                                        <input
                                            type="text"
                                            prop:value=move || form_database.get()
                                            on:input=move |ev| form_database.set(event_target_value(&ev))
                                            placeholder=move || if is_sqlite() { "/path/to/database.db" } else { "mydb" }
                                            style="
                                                width: 100%;
                                                padding: 6px 8px;
                                                background: #1e1e1e;
                                                border: 1px solid #3e3e3e;
                                                border-radius: 4px;
                                                color: #cccccc;
                                                font-size: 12px;
                                            "
                                        />
                                    </div>

                                    // Username (not for SQLite)
                                    {move || {
                                        if !is_sqlite() {
                                            view! {
                                                <div>
                                                    <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                                        "Username"
                                                    </label>
                                                    <input
                                                        type="text"
                                                        prop:value=move || form_username.get()
                                                        on:input=move |ev| form_username.set(event_target_value(&ev))
                                                        placeholder="postgres"
                                                        style="
                                                            width: 100%;
                                                            padding: 6px 8px;
                                                            background: #1e1e1e;
                                                            border: 1px solid #3e3e3e;
                                                            border-radius: 4px;
                                                            color: #cccccc;
                                                            font-size: 12px;
                                                        "
                                                    />
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    // Password (not for SQLite)
                                    {move || {
                                        if !is_sqlite() {
                                            view! {
                                                <div>
                                                    <label style="display: block; margin-bottom: 6px; font-size: 12px; color: #cccccc;">
                                                        "Password"
                                                    </label>
                                                    <input
                                                        type="password"
                                                        prop:value=move || form_password.get()
                                                        on:input=move |ev| form_password.set(event_target_value(&ev))
                                                        placeholder="••••••••"
                                                        style="
                                                            width: 100%;
                                                            padding: 6px 8px;
                                                            background: #1e1e1e;
                                                            border: 1px solid #3e3e3e;
                                                            border-radius: 4px;
                                                            color: #cccccc;
                                                            font-size: 12px;
                                                        "
                                                    />
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    // SSL (not for SQLite)
                                    {move || {
                                        if !is_sqlite() {
                                            view! {
                                                <div style="display: flex; align-items: center; gap: 8px;">
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || form_ssl.get()
                                                        on:change=move |ev| form_ssl.set(event_target_checked(&ev))
                                                        style="cursor: pointer;"
                                                    />
                                                    <label style="font-size: 12px; color: #cccccc; cursor: pointer;">
                                                        "Use SSL"
                                                    </label>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    // Test Result Display
                                    {move || {
                                        if let Some(result) = test_result.get() {
                                            let (color, icon) = if result.success {
                                                ("#4EC9B0", "codicon-pass")
                                            } else {
                                                ("#F48771", "codicon-error")
                                            };
                                            view! {
                                                <div style=format!(
                                                    "padding: 8px 12px; background: rgba(255,255,255,0.05); border-left: 3px solid {}; border-radius: 4px; font-size: 11px;",
                                                    color
                                                )>
                                                    <div style="display: flex; align-items: center; gap: 6px;">
                                                        <i class=format!("codicon {}", icon) style=format!("color: {};", color)></i>
                                                        <span style=format!("color: {};", color)>{result.message.clone()}</span>
                                                    </div>
                                                    {result.server_version.as_ref().map(|v| view! {
                                                        <div style="margin-top: 4px; color: #858585;">
                                                            {format!("Version: {}", v)}
                                                        </div>
                                                    })}
                                                    {result.latency_ms.map(|ms| view! {
                                                        <div style="margin-top: 4px; color: #858585;">
                                                            {format!("Latency: {}ms", ms)}
                                                        </div>
                                                    })}
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}
                                </div>

                                <div style="
                                    padding: 12px 16px;
                                    border-top: 1px solid #3e3e3e;
                                    display: flex;
                                    justify-content: space-between;
                                    gap: 8px;
                                ">
                                    <button
                                        on:click=move |_| {
                                            is_testing.set(true);
                                            test_result.set(None);

                                            let conn = DbConnection {
                                                id: String::new(),
                                                name: form_name.get(),
                                                db_type: form_db_type.get(),
                                                host: if is_sqlite() { None } else { Some(form_host.get()) },
                                                port: if is_sqlite() { None } else { form_port.get().parse().ok() },
                                                database: form_database.get(),
                                                username: if is_sqlite() { None } else { Some(form_username.get()) },
                                                password: if is_sqlite() { None } else { Some(form_password.get()) },
                                                ssl: form_ssl.get(),
                                                created_at: 0,
                                                last_used: None,
                                            };

                                            leptos::task::spawn_local(async move {
                                                match crate::tauri_bindings_database::db_test_connection(conn).await {
                                                    Ok(result) => {
                                                        test_result.set(Some(result));
                                                        is_testing.set(false);
                                                    }
                                                    Err(e) => {
                                                        test_result.set(Some(ConnectionTestResult {
                                                            success: false,
                                                            message: format!("Error: {}", e),
                                                            latency_ms: None,
                                                            server_version: None,
                                                        }));
                                                        is_testing.set(false);
                                                    }
                                                }
                                            });
                                        }
                                        disabled=move || is_testing.get()
                                        style="
                                            background: #0e639c;
                                            border: none;
                                            color: #ffffff;
                                            padding: 6px 12px;
                                            cursor: pointer;
                                            font-size: 12px;
                                            border-radius: 4px;
                                        "
                                    >
                                        {move || if is_testing.get() { "Testing..." } else { "Test Connection" }}
                                    </button>

                                    <div style="display: flex; gap: 8px;">
                                        <button
                                            on:click=move |_| {
                                                show_add_dialog.set(false);
                                                show_edit_dialog.set(false);
                                                test_result.set(None);
                                            }
                                            style="
                                                background: #3c3c3c;
                                                border: 1px solid #858585;
                                                color: #ffffff;
                                                padding: 6px 12px;
                                                cursor: pointer;
                                                font-size: 12px;
                                                border-radius: 4px;
                                            "
                                        >
                                            "Cancel"
                                        </button>
                                        <button
                                            on:click=move |_| {
                                                let conn = DbConnection {
                                                    id: if show_edit_dialog.get() {
                                                        edit_connection.get().map(|c| c.id).unwrap_or_else(|| {
                                                            SystemTime::now()
                                                                .duration_since(UNIX_EPOCH)
                                                                .unwrap()
                                                                .as_secs()
                                                                .to_string()
                                                        })
                                                    } else {
                                                        SystemTime::now()
                                                            .duration_since(UNIX_EPOCH)
                                                            .unwrap()
                                                            .as_secs()
                                                            .to_string()
                                                    },
                                                    name: form_name.get(),
                                                    db_type: form_db_type.get(),
                                                    host: if is_sqlite() { None } else { Some(form_host.get()) },
                                                    port: if is_sqlite() { None } else { form_port.get().parse().ok() },
                                                    database: form_database.get(),
                                                    username: if is_sqlite() { None } else { Some(form_username.get()) },
                                                    password: if is_sqlite() { None } else { Some(form_password.get()) },
                                                    ssl: form_ssl.get(),
                                                    created_at: SystemTime::now()
                                                        .duration_since(UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_secs() as i64,
                                                    last_used: None,
                                                };

                                                leptos::task::spawn_local(async move {
                                                    let result = if show_edit_dialog.get() {
                                                        crate::tauri_bindings_database::db_update_connection(conn).await
                                                    } else {
                                                        crate::tauri_bindings_database::db_add_connection(conn).await
                                                    };

                                                    match result {
                                                        Ok(_) => {
                                                            leptos::logging::log!("✅ Connection saved");
                                                            show_add_dialog.set(false);
                                                            show_edit_dialog.set(false);
                                                            test_result.set(None);
                                                            reload_connections();
                                                        }
                                                        Err(e) => leptos::logging::error!("Save error: {}", e),
                                                    }
                                                });
                                            }
                                            style="
                                                background: #0e639c;
                                                border: none;
                                                color: #ffffff;
                                                padding: 6px 12px;
                                                cursor: pointer;
                                                font-size: 12px;
                                                border-radius: 4px;
                                            "
                                        >
                                            "Save"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}
