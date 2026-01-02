//! Database layer for persistent storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::path::PathBuf;

/// Database connection pool
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    db_type: DatabaseType,
}

#[derive(Clone, Debug, PartialEq)]
enum DatabaseType {
    Postgres,
    Sqlite,
}

// Macro to create tables with database-specific SQL
macro_rules! exec_db_specific {
    ($pool:expr, $db_type:expr, $pg_sql:expr, $sqlite_sql:expr) => {
        {
            let sql = match $db_type {
                DatabaseType::Postgres => $pg_sql,
                DatabaseType::Sqlite => $sqlite_sql,
            };
            sqlx::query(sql).execute($pool).await?
        }
    };
}

// Helper functions for DateTime handling with AnyPool
fn datetime_to_string(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn string_to_datetime(s: &str) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc))
}

impl Database {
    /// Initialize database with schema
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // Determine database type from URL
        let db_type = if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            DatabaseType::Postgres
        } else {
            DatabaseType::Sqlite
        };

        tracing::info!("Connected to {:?} database", db_type);

        // Create tables with database-specific SQL
        let create_users_sql = match db_type {
            DatabaseType::Postgres => r#"
                CREATE TABLE IF NOT EXISTS users (
                    id SERIAL PRIMARY KEY,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL,
                    role TEXT NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                )
            "#,
            DatabaseType::Sqlite => r#"
                CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL,
                    role TEXT NOT NULL,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                )
            "#,
        };
        sqlx::query(create_users_sql).execute(&pool).await?;

        // Sessions table (no AUTO INCREMENT needed, TEXT PRIMARY KEY)
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                project_root TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                last_activity TIMESTAMPTZ NOT NULL,
                shared BOOLEAN NOT NULL DEFAULT FALSE,
                share_url TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                project_root TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                last_activity DATETIME NOT NULL,
                shared INTEGER NOT NULL DEFAULT 0,
                share_url TEXT
            )"#
        );

        // Chat messages table
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS chat_messages (
                id SERIAL PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )"#,
            r#"CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )"#
        );

        // Session tokens table
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS session_tokens (
                token TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                expires_at TIMESTAMPTZ NOT NULL,
                FOREIGN KEY (username) REFERENCES users(username) ON DELETE CASCADE
            )"#,
            r#"CREATE TABLE IF NOT EXISTS session_tokens (
                token TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME NOT NULL,
                FOREIGN KEY (username) REFERENCES users(username) ON DELETE CASCADE
            )"#
        );

        // Recent projects table
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS recent_projects (
                id SERIAL PRIMARY KEY,
                project_root TEXT NOT NULL,
                last_accessed TIMESTAMPTZ NOT NULL,
                session_count INTEGER NOT NULL DEFAULT 1
            )"#,
            r#"CREATE TABLE IF NOT EXISTS recent_projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_root TEXT NOT NULL,
                last_accessed DATETIME NOT NULL,
                session_count INTEGER NOT NULL DEFAULT 1
            )"#
        );

        // Workflow templates table
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS workflow_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                is_preset BOOLEAN NOT NULL DEFAULT FALSE,
                version INTEGER NOT NULL DEFAULT 1,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )"#,
            r#"CREATE TABLE IF NOT EXISTS workflow_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                is_preset INTEGER NOT NULL DEFAULT 0,
                version INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#
        );

        // Workflow template versions table
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS workflow_template_versions (
                id SERIAL PRIMARY KEY,
                template_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                change_description TEXT,
                created_by TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                FOREIGN KEY (template_id) REFERENCES workflow_templates(id) ON DELETE CASCADE,
                UNIQUE(template_id, version)
            )"#,
            r#"CREATE TABLE IF NOT EXISTS workflow_template_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                change_description TEXT,
                created_by TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (template_id) REFERENCES workflow_templates(id) ON DELETE CASCADE,
                UNIQUE(template_id, version)
            )"#
        );

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_executions (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                pipeline_id TEXT NOT NULL,
                pipeline_name TEXT NOT NULL,
                status TEXT NOT NULL,
                start_time DATETIME NOT NULL,
                end_time DATETIME,
                loop_count INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                execution_log TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_snapshots (
                snapshot_id TEXT PRIMARY KEY,
                execution_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                node_name TEXT NOT NULL,
                snapshot_data TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                FOREIGN KEY (execution_id) REFERENCES workflow_executions(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // BerryChat tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                topic TEXT,
                created_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_messages (
                id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                user TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                parent_message_id TEXT,
                FOREIGN KEY (channel_id) REFERENCES berrychat_channels(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_message_id) REFERENCES berrychat_messages(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_reactions (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL,
                user TEXT NOT NULL,
                emoji TEXT NOT NULL,
                FOREIGN KEY (message_id) REFERENCES berrychat_messages(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                display_name TEXT,
                email TEXT,
                avatar_url TEXT,
                status TEXT DEFAULT 'offline',
                status_message TEXT,
                created_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_channel_members (
                id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                joined_at DATETIME NOT NULL,
                role TEXT DEFAULT 'member',
                FOREIGN KEY (channel_id) REFERENCES berrychat_channels(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES berrychat_users(id) ON DELETE CASCADE,
                UNIQUE(channel_id, user_id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_direct_messages (
                id TEXT PRIMARY KEY,
                user1_id TEXT NOT NULL,
                user2_id TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                FOREIGN KEY (user1_id) REFERENCES berrychat_users(id) ON DELETE CASCADE,
                FOREIGN KEY (user2_id) REFERENCES berrychat_users(id) ON DELETE CASCADE,
                UNIQUE(user1_id, user2_id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS berrychat_file_attachments (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL,
                filename TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                mime_type TEXT,
                uploaded_at DATETIME NOT NULL,
                FOREIGN KEY (message_id) REFERENCES berrychat_messages(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS slack_mentions (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                FOREIGN KEY (message_id) REFERENCES berrychat_messages(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES berrychat_users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // WebRTC tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS webrtc_calls (
                id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                initiator TEXT NOT NULL,
                call_type TEXT NOT NULL,
                started_at DATETIME NOT NULL,
                ended_at DATETIME,
                FOREIGN KEY (channel_id) REFERENCES berrychat_channels(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS webrtc_participants (
                id TEXT PRIMARY KEY,
                call_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                joined_at DATETIME NOT NULL,
                left_at DATETIME,
                FOREIGN KEY (call_id) REFERENCES webrtc_calls(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES berrychat_users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Virtual Office tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS virtual_office_spaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                width INTEGER NOT NULL DEFAULT 50,
                height INTEGER NOT NULL DEFAULT 50,
                tile_size INTEGER NOT NULL DEFAULT 32,
                background_color TEXT DEFAULT '#1a1a1a',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS virtual_office_users (
                id TEXT PRIMARY KEY,
                space_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                username TEXT NOT NULL,
                x INTEGER NOT NULL DEFAULT 5,
                y INTEGER NOT NULL DEFAULT 5,
                direction TEXT NOT NULL DEFAULT 'down',
                avatar TEXT DEFAULT '👤',
                status TEXT DEFAULT 'online',
                last_update DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (space_id) REFERENCES virtual_office_spaces(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS virtual_office_objects (
                id TEXT PRIMARY KEY,
                space_id TEXT NOT NULL,
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                width INTEGER NOT NULL DEFAULT 1,
                height INTEGER NOT NULL DEFAULT 1,
                object_type TEXT NOT NULL,
                properties TEXT,
                walkable INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (space_id) REFERENCES virtual_office_spaces(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Model settings table
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS model_settings (
                id SERIAL PRIMARY KEY,
                session_id TEXT NOT NULL,
                task_type TEXT NOT NULL,
                model_name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(session_id, task_type),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )"#,
            r#"CREATE TABLE IF NOT EXISTS model_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                task_type TEXT NOT NULL,
                model_name TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(session_id, task_type),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )"#
        );

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_model_settings_session_id ON model_settings(session_id)")
            .execute(&pool)
            .await?;

        // API Keys table (encrypted storage)
        exec_db_specific!(&pool, &db_type,
            r#"CREATE TABLE IF NOT EXISTS api_keys (
                id SERIAL PRIMARY KEY,
                session_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                encrypted_key TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(session_id, provider),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )"#,
            r#"CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                encrypted_key TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(session_id, provider),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )"#
        );

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_session_id ON api_keys(session_id)")
            .execute(&pool)
            .await?;

        // Remote connections table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS remote_connections (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 22,
                username TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_connected DATETIME,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_remote_connections_session_id ON remote_connections(session_id)")
            .execute(&pool)
            .await?;

        // Create default admin user if not exists
        let admin_exists: bool = sqlx::query("SELECT COUNT(*) as count FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await?
            .get::<i64, _>("count") > 0;

        if !admin_exists {
            let password_hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST)?;
            sqlx::query("INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)")
                .bind("admin")
                .bind(&password_hash)
                .bind("admin")
                .execute(&pool)
                .await?;
        }

        let db = Self { pool, db_type };

        // Initialize preset workflow templates
        db.init_preset_templates().await?;

        // Initialize BerryChat defaults
        db.init_slack_defaults().await?;

        // Initialize Virtual Office defaults
        db.init_virtual_office_defaults().await?;

        Ok(db)
    }

    /// Get reference to the database pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // User operations
    pub async fn create_user(&self, username: &str, password: &str, role: &str) -> anyhow::Result<()> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        sqlx::query("INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)")
            .bind(username)
            .bind(&password_hash)
            .bind(role)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn verify_user(&self, username: &str, password: &str) -> anyhow::Result<bool> {
        let row = sqlx::query("SELECT password_hash FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let password_hash: String = row.get("password_hash");
            Ok(bcrypt::verify(password, &password_hash)?)
        } else {
            Ok(false)
        }
    }

    pub async fn create_session_token(&self, username: String) -> anyhow::Result<String> {
        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::days(7);

        sqlx::query("INSERT INTO session_tokens (token, username, expires_at) VALUES (?, ?, ?)")
            .bind(&token)
            .bind(&username)
            .bind(datetime_to_string(expires_at))
            .execute(&self.pool)
            .await?;

        Ok(token)
    }

    pub async fn verify_session_token(&self, token: &str) -> anyhow::Result<Option<String>> {
        let row = sqlx::query(
            "SELECT username FROM session_tokens WHERE token = ? AND expires_at > CURRENT_TIMESTAMP"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("username")))
    }

    pub async fn destroy_session_token(&self, token: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM session_tokens WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Session operations
    pub async fn create_session(&self, session_id: &str, project_root: &PathBuf) -> anyhow::Result<()> {
        let now = datetime_to_string(Utc::now());
        sqlx::query(
            "INSERT INTO sessions (id, project_root, created_at, last_activity) VALUES (?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(project_root.to_string_lossy().as_ref())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<SessionData>> {
        let row = sqlx::query(
            "SELECT id, project_root, created_at, last_activity, shared, share_url FROM sessions WHERE id = ?"
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(SessionData {
                id: row.get("id"),
                project_root: PathBuf::from(row.get::<String, _>("project_root")),
                created_at: string_to_datetime(&row.get::<String, _>("created_at"))?,
                last_activity: string_to_datetime(&row.get::<String, _>("last_activity"))?,
                shared: row.get::<i64, _>("shared") != 0,
                share_url: row.get("share_url"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_session_activity(&self, session_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE sessions SET last_activity = ? WHERE id = ?")
            .bind(datetime_to_string(Utc::now()))
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn enable_sharing(&self, session_id: &str, share_url: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE sessions SET shared = 1, share_url = ? WHERE id = ?")
            .bind(share_url)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Chat message operations
    pub async fn add_chat_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO chat_messages (session_id, role, content, timestamp) VALUES (?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(datetime_to_string(Utc::now()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_chat_history(&self, session_id: &str) -> anyhow::Result<Vec<ChatMessageData>> {
        let rows = sqlx::query(
            "SELECT role, content, timestamp FROM chat_messages WHERE session_id = ? ORDER BY timestamp ASC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Ok::<_, anyhow::Error>(ChatMessageData {
                role: row.get("role"),
                content: row.get("content"),
                timestamp: string_to_datetime(&row.get::<String, _>("timestamp"))?,
            }))
            .collect::<anyhow::Result<Vec<_>>>()?)
    }

    pub async fn cleanup_expired_tokens(&self) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM session_tokens WHERE expires_at < CURRENT_TIMESTAMP")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Recent projects operations
    pub async fn add_recent_project(&self, project_root: &str) -> anyhow::Result<()> {
        // Check if project already exists
        let existing: Option<i64> = sqlx::query("SELECT id FROM recent_projects WHERE project_root = ?")
            .bind(project_root)
            .fetch_optional(&self.pool)
            .await?
            .map(|row| row.get("id"));

        if let Some(id) = existing {
            // Update existing project
            sqlx::query("UPDATE recent_projects SET last_accessed = ?, session_count = session_count + 1 WHERE id = ?")
                .bind(datetime_to_string(Utc::now()))
                .bind(id)
                .execute(&self.pool)
                .await?;
        } else {
            // Insert new project
            sqlx::query("INSERT INTO recent_projects (project_root, last_accessed, session_count) VALUES (?, ?, 1)")
                .bind(project_root)
                .bind(datetime_to_string(Utc::now()))
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn get_recent_projects(&self, limit: i32) -> anyhow::Result<Vec<RecentProject>> {
        let rows = sqlx::query(
            "SELECT project_root, last_accessed, session_count FROM recent_projects ORDER BY last_accessed DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows
            .into_iter()
            .map(|row| Ok::<_, anyhow::Error>(RecentProject {
                project_root: row.get("project_root"),
                last_accessed: string_to_datetime(&row.get::<String, _>("last_accessed"))?,
                session_count: row.get("session_count"),
            }))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    // Workflow template operations
    pub async fn create_workflow_template(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        definition: &str,
        is_preset: bool,
    ) -> anyhow::Result<()> {
        // Insert template with version 1
        sqlx::query(
            "INSERT INTO workflow_templates (id, name, description, definition, is_preset, version) VALUES (?, ?, ?, ?, ?, 1)"
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(definition)
        .bind(if is_preset { 1 } else { 0 })
        .execute(&self.pool)
        .await?;

        // Save initial version to history
        sqlx::query(
            "INSERT INTO workflow_template_versions (template_id, version, name, description, definition, change_description) VALUES (?, 1, ?, ?, ?, 'Initial version')"
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(definition)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_workflow_template(&self, id: &str) -> anyhow::Result<Option<WorkflowTemplate>> {
        let row = sqlx::query(
            "SELECT id, name, description, definition, is_preset, version, created_at, updated_at FROM workflow_templates WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(WorkflowTemplate {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                definition: row.get("definition"),
                is_preset: row.get::<i64, _>("is_preset") != 0,
                version: row.get("version"),
                created_at: string_to_datetime(&row.get::<String, _>("created_at"))?,
                updated_at: string_to_datetime(&row.get::<String, _>("updated_at"))?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_workflow_templates(&self) -> anyhow::Result<Vec<WorkflowTemplate>> {
        let rows = sqlx::query(
            "SELECT id, name, description, definition, is_preset, version, created_at, updated_at FROM workflow_templates ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        rows
            .into_iter()
            .map(|row| Ok::<_, anyhow::Error>(WorkflowTemplate {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                definition: row.get("definition"),
                is_preset: row.get::<i64, _>("is_preset") != 0,
                version: row.get("version"),
                created_at: string_to_datetime(&row.get::<String, _>("created_at"))?,
                updated_at: string_to_datetime(&row.get::<String, _>("updated_at"))?,
            }))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    pub async fn update_workflow_template(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        definition: &str,
        change_description: Option<&str>,
    ) -> anyhow::Result<()> {
        // Get current template to save to history
        let current = self.get_workflow_template(id).await?
            .ok_or_else(|| anyhow::anyhow!("Template not found"))?;

        let new_version = current.version + 1;

        // Save current version to history
        sqlx::query(
            "INSERT INTO workflow_template_versions (template_id, version, name, description, definition, change_description) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(new_version)
        .bind(name)
        .bind(description)
        .bind(definition)
        .bind(change_description)
        .execute(&self.pool)
        .await?;

        // Update template with new version
        sqlx::query(
            "UPDATE workflow_templates SET name = ?, description = ?, definition = ?, version = ?, updated_at = ? WHERE id = ?"
        )
        .bind(name)
        .bind(description)
        .bind(definition)
        .bind(new_version)
        .bind(datetime_to_string(Utc::now()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_workflow_template(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM workflow_templates WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Workflow execution log operations
    pub async fn create_workflow_execution(
        &self,
        id: &str,
        session_id: &str,
        pipeline_id: &str,
        pipeline_name: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO workflow_executions (id, session_id, pipeline_id, pipeline_name, status, start_time, loop_count) VALUES (?, ?, ?, ?, 'running', ?, 0)"
        )
        .bind(id)
        .bind(session_id)
        .bind(pipeline_id)
        .bind(pipeline_name)
        .bind(datetime_to_string(Utc::now()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn search_workflow_executions(
        &self,
        session_id: Option<&str>,
        pipeline_id: Option<&str>,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> anyhow::Result<Vec<WorkflowExecution>> {
        let mut query = String::from(
            "SELECT id, session_id, pipeline_id, pipeline_name, status, start_time, end_time, loop_count, error_message, execution_log FROM workflow_executions WHERE 1=1"
        );

        let mut bindings: Vec<String> = Vec::new();

        if let Some(sid) = session_id {
            query.push_str(" AND session_id = ?");
            bindings.push(sid.to_string());
        }

        if let Some(pid) = pipeline_id {
            query.push_str(" AND pipeline_id = ?");
            bindings.push(pid.to_string());
        }

        if let Some(st) = status {
            query.push_str(" AND status = ?");
            bindings.push(st.to_string());
        }

        query.push_str(" ORDER BY start_time DESC LIMIT ? OFFSET ?");

        let mut sqlx_query = sqlx::query(&query);
        for binding in &bindings {
            sqlx_query = sqlx_query.bind(binding);
        }
        sqlx_query = sqlx_query.bind(limit).bind(offset);

        let rows = sqlx_query.fetch_all(&self.pool).await?;

        rows
            .into_iter()
            .map(|row| Ok::<_, anyhow::Error>(WorkflowExecution {
                id: row.get("id"),
                session_id: row.get("session_id"),
                pipeline_id: row.get("pipeline_id"),
                pipeline_name: row.get("pipeline_name"),
                status: row.get("status"),
                start_time: string_to_datetime(&row.get::<String, _>("start_time"))?,
                end_time: row.get::<Option<String>, _>("end_time")
                    .map(|s| string_to_datetime(&s))
                    .transpose()?,
                loop_count: row.get::<i64, _>("loop_count") as usize,
                error_message: row.get("error_message"),
                execution_log: row.get("execution_log"),
            }))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    pub async fn get_workflow_execution(&self, id: &str) -> anyhow::Result<Option<WorkflowExecution>> {
        let row = sqlx::query(
            "SELECT id, session_id, pipeline_id, pipeline_name, status, start_time, end_time, loop_count, error_message, execution_log FROM workflow_executions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(WorkflowExecution {
                id: row.get("id"),
                session_id: row.get("session_id"),
                pipeline_id: row.get("pipeline_id"),
                pipeline_name: row.get("pipeline_name"),
                status: row.get("status"),
                start_time: string_to_datetime(&row.get::<String, _>("start_time"))?,
                end_time: row.get::<Option<String>, _>("end_time")
                    .map(|s| string_to_datetime(&s))
                    .transpose()?,
                loop_count: row.get::<i64, _>("loop_count") as usize,
                error_message: row.get("error_message"),
                execution_log: row.get("execution_log"),
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub project_root: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub shared: bool,
    pub share_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageData {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub project_root: String,
    pub last_accessed: DateTime<Utc>,
    pub session_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub definition: String,
    pub is_preset: bool,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateVersion {
    pub id: i64,
    pub template_id: String,
    pub version: i64,
    pub name: String,
    pub description: Option<String>,
    pub definition: String,
    pub change_description: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub session_id: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub status: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub loop_count: usize,
    pub error_message: Option<String>,
    pub execution_log: Option<String>,
}

impl Database {
    /// プリセットテンプレートを初期化
    async fn init_preset_templates(&self) -> anyhow::Result<()> {
        // TDD Loop preset
        let tdd_preset_exists: bool = sqlx::query("SELECT COUNT(*) as count FROM workflow_templates WHERE id = 'tdd-loop'")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count") > 0;

        if !tdd_preset_exists {
            let tdd_pipeline = crate::berrycode::pipeline::create_tdd_loop_preset();
            let tdd_definition = serde_json::to_string(&tdd_pipeline)?;

            self.create_workflow_template(
                "tdd-loop",
                "TDD Loop",
                Some("Test → Fix → Re-Test workflow"),
                &tdd_definition,
                true, // is_preset
            ).await?;

            tracing::info!("Initialized TDD Loop preset template");
        }

        // Full Dev preset
        let full_dev_preset_exists: bool = sqlx::query("SELECT COUNT(*) as count FROM workflow_templates WHERE id = 'full-dev'")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count") > 0;

        if !full_dev_preset_exists {
            let full_dev_pipeline = crate::berrycode::pipeline::create_full_dev_pipeline();
            let full_dev_definition = serde_json::to_string(&full_dev_pipeline)?;

            self.create_workflow_template(
                "full-dev",
                "Full Development",
                Some("Design → Implement → Test → Fix → Refactor workflow"),
                &full_dev_definition,
                true, // is_preset
            ).await?;

            tracing::info!("Initialized Full Development preset template");
        }

        Ok(())
    }

    /// ワークフロー実行を保存
    pub async fn save_workflow_execution(
        &self,
        id: &str,
        session_id: &str,
        pipeline_id: &str,
        pipeline_name: &str,
        status: &str,
        error_message: Option<&str>,
        execution_log: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO workflow_executions
             (id, session_id, pipeline_id, pipeline_name, status, start_time, loop_count, error_message, execution_log)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(session_id)
        .bind(pipeline_id)
        .bind(pipeline_name)
        .bind(status)
        .bind(datetime_to_string(chrono::Utc::now()))
        .bind(0)
        .bind(error_message)
        .bind(execution_log)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// ワークフロー実行を更新
    pub async fn update_workflow_execution(
        &self,
        id: &str,
        status: &str,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        loop_count: usize,
        error_message: Option<&str>,
        execution_log: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE workflow_executions
             SET status = ?, end_time = ?, loop_count = ?, error_message = ?, execution_log = ?
             WHERE id = ?"
        )
        .bind(status)
        .bind(end_time.map(datetime_to_string))
        .bind(loop_count as i64)
        .bind(error_message)
        .bind(execution_log)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// スナップショットを保存
    pub async fn save_snapshot(
        &self,
        snapshot_id: &str,
        execution_id: &str,
        node_id: &str,
        node_name: &str,
        snapshot_data: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO workflow_snapshots (snapshot_id, execution_id, node_id, node_name, snapshot_data, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(snapshot_id)
        .bind(execution_id)
        .bind(node_id)
        .bind(node_name)
        .bind(snapshot_data)
        .bind(datetime_to_string(Utc::now()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// スナップショットを取得
    pub async fn get_snapshot(&self, snapshot_id: &str) -> anyhow::Result<Option<String>> {
        let row = sqlx::query("SELECT snapshot_data FROM workflow_snapshots WHERE snapshot_id = ?")
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("snapshot_data")))
    }

    /// 実行IDに紐づくスナップショット一覧を取得
    pub async fn list_snapshots_by_execution(&self, execution_id: &str) -> anyhow::Result<Vec<SnapshotInfo>> {
        let rows = sqlx::query(
            "SELECT snapshot_id, execution_id, node_id, node_name, timestamp
             FROM workflow_snapshots
             WHERE execution_id = ?
             ORDER BY timestamp DESC"
        )
        .bind(execution_id)
        .fetch_all(&self.pool)
        .await?;

        let snapshots = rows.iter().map(|row| Ok::<_, anyhow::Error>(SnapshotInfo {
            snapshot_id: row.get("snapshot_id"),
            execution_id: row.get("execution_id"),
            node_id: row.get("node_id"),
            node_name: row.get("node_name"),
            timestamp: string_to_datetime(&row.get::<String, _>("timestamp"))?,
        })).collect::<anyhow::Result<Vec<_>>>()?;

        Ok(snapshots)
    }

    /// スナップショットを削除
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM workflow_snapshots WHERE snapshot_id = ?")
            .bind(snapshot_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// テンプレートのバージョン一覧を取得
    pub async fn list_template_versions(&self, template_id: &str) -> anyhow::Result<Vec<WorkflowTemplateVersion>> {
        let rows = sqlx::query(
            "SELECT id, template_id, version, name, description, definition, change_description, created_by, created_at
             FROM workflow_template_versions
             WHERE template_id = ?
             ORDER BY version DESC"
        )
        .bind(template_id)
        .fetch_all(&self.pool)
        .await?;

        rows
            .into_iter()
            .map(|row| Ok::<_, anyhow::Error>(WorkflowTemplateVersion {
                id: row.get("id"),
                template_id: row.get("template_id"),
                version: row.get("version"),
                name: row.get("name"),
                description: row.get("description"),
                definition: row.get("definition"),
                change_description: row.get("change_description"),
                created_by: row.get("created_by"),
                created_at: string_to_datetime(&row.get::<String, _>("created_at"))?,
            }))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    /// 特定バージョンを取得
    pub async fn get_template_version(&self, template_id: &str, version: i64) -> anyhow::Result<Option<WorkflowTemplateVersion>> {
        let row = sqlx::query(
            "SELECT id, template_id, version, name, description, definition, change_description, created_by, created_at
             FROM workflow_template_versions
             WHERE template_id = ? AND version = ?"
        )
        .bind(template_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(WorkflowTemplateVersion {
                id: row.get("id"),
                template_id: row.get("template_id"),
                version: row.get("version"),
                name: row.get("name"),
                description: row.get("description"),
                definition: row.get("definition"),
                change_description: row.get("change_description"),
                created_by: row.get("created_by"),
                created_at: string_to_datetime(&row.get::<String, _>("created_at"))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 過去のバージョンに復元
    pub async fn restore_template_version(&self, template_id: &str, version: i64) -> anyhow::Result<()> {
        // Get the version to restore
        let old_version = self.get_template_version(template_id, version).await?
            .ok_or_else(|| anyhow::anyhow!("Version not found"))?;

        // Get current template
        let current = self.get_workflow_template(template_id).await?
            .ok_or_else(|| anyhow::anyhow!("Template not found"))?;

        let new_version = current.version + 1;

        // Save current state to history before restoring
        sqlx::query(
            "INSERT INTO workflow_template_versions (template_id, version, name, description, definition, change_description) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(template_id)
        .bind(new_version)
        .bind(&current.name)
        .bind(&current.description)
        .bind(&current.definition)
        .bind(format!("Restored from version {}", version))
        .execute(&self.pool)
        .await?;

        // Restore old version as current
        sqlx::query(
            "UPDATE workflow_templates SET name = ?, description = ?, definition = ?, version = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&old_version.name)
        .bind(&old_version.description)
        .bind(&old_version.definition)
        .bind(new_version)
        .bind(datetime_to_string(Utc::now()))
        .bind(template_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Initialize default BerryChat users and channels
    async fn init_slack_defaults(&self) -> anyhow::Result<()> {
        // Create default user if not exists
        let default_user_exists: bool = sqlx::query("SELECT COUNT(*) as count FROM berrychat_users WHERE username = 'admin'")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count") > 0;

        let default_user_id = if !default_user_exists {
            let user_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO berrychat_users (id, username, display_name, email, status, created_at) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&user_id)
            .bind("admin")
            .bind("Admin User")
            .bind("admin@berrycode.local")
            .bind("online")
            .bind(datetime_to_string(chrono::Utc::now()))
            .execute(&self.pool)
            .await?;
            user_id
        } else {
            sqlx::query("SELECT id FROM berrychat_users WHERE username = 'admin'")
                .fetch_one(&self.pool)
                .await?
                .get::<String, _>("id")
        };

        // Create default channels
        let channels = vec![
            ("general", "General discussion"),
            ("random", "Random chatter"),
            ("dev", "Development discussions"),
        ];

        for (name, topic) in channels {
            let channel_exists: bool = sqlx::query("SELECT COUNT(*) as count FROM berrychat_channels WHERE name = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await?
                .get::<i64, _>("count") > 0;

            if !channel_exists {
                let channel_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO berrychat_channels (id, name, topic, created_at) VALUES (?, ?, ?, ?)"
                )
                .bind(&channel_id)
                .bind(name)
                .bind(topic)
                .bind(datetime_to_string(chrono::Utc::now()))
                .execute(&self.pool)
                .await?;

                // Add admin user to the channel
                let member_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO berrychat_channel_members (id, channel_id, user_id, joined_at, role) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&member_id)
                .bind(&channel_id)
                .bind(&default_user_id)
                .bind(datetime_to_string(chrono::Utc::now()))
                .bind("admin")
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Initialize default Virtual Office space
    async fn init_virtual_office_defaults(&self) -> anyhow::Result<()> {
        // Create default office space if not exists
        let default_space_exists: bool = sqlx::query("SELECT COUNT(*) as count FROM virtual_office_spaces WHERE name = 'Main Office'")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count") > 0;

        if !default_space_exists {
            let space_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO virtual_office_spaces (id, name, width, height, tile_size, background_color) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&space_id)
            .bind("Main Office")
            .bind(50)
            .bind(50)
            .bind(32)
            .bind("#1a1a1a")
            .execute(&self.pool)
            .await?;

            // Add some default objects (walls, desks, meeting rooms)
            let objects: Vec<(String, i32, i32, i32, i32, &str, &str, i32)> = vec![
                // Meeting room 1 (top-left)
                (space_id.clone(), 5, 5, 8, 1, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 5, 5, 1, 6, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 5, 10, 8, 1, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 12, 5, 1, 6, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 6, 6, 6, 4, "room", "{\"color\":\"#2d3748\"}", 1),

                // Meeting room 2 (top-right)
                (space_id.clone(), 35, 5, 8, 1, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 35, 5, 1, 6, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 35, 10, 8, 1, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 42, 5, 1, 6, "wall", "{\"color\":\"#555\"}", 0),
                (space_id.clone(), 36, 6, 6, 4, "room", "{\"color\":\"#2d3748\"}", 1),

                // Desks area (center)
                (space_id.clone(), 15, 15, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 19, 15, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 23, 15, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 27, 15, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 31, 15, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),

                (space_id.clone(), 15, 20, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 19, 20, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 23, 20, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 27, 20, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),
                (space_id.clone(), 31, 20, 2, 1, "desk", "{\"color\":\"#8b4513\"}", 0),

                // Lounge area (bottom)
                (space_id.clone(), 20, 35, 3, 3, "couch", "{\"color\":\"#4a5568\"}", 0),
                (space_id.clone(), 27, 35, 3, 3, "couch", "{\"color\":\"#4a5568\"}", 0),
                (space_id.clone(), 23, 37, 2, 2, "table", "{\"color\":\"#2d3748\"}", 0),
            ];

            for (sid, x, y, w, h, otype, props, walkable) in objects {
                let obj_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO virtual_office_objects (id, space_id, x, y, width, height, object_type, properties, walkable) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&obj_id)
                .bind(&sid)
                .bind(x)
                .bind(y)
                .bind(w)
                .bind(h)
                .bind(otype)
                .bind(props)
                .bind(walkable)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Initialize default model settings for a session
    pub async fn init_default_model_settings(&self, session_id: &str) -> anyhow::Result<()> {
        use crate::berrycode::web::model_settings_api::get_default_settings;

        let default_settings = get_default_settings();

        for (task_type, model_name) in default_settings {
            // Check if setting already exists
            let exists: bool = sqlx::query(
                "SELECT COUNT(*) as count FROM model_settings WHERE session_id = ? AND task_type = ?"
            )
            .bind(session_id)
            .bind(&task_type)
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count") > 0;

            if !exists {
                sqlx::query(
                    "INSERT INTO model_settings (session_id, task_type, model_name) VALUES (?, ?, ?)"
                )
                .bind(session_id)
                .bind(&task_type)
                .bind(&model_name)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Get model settings for a session
    pub async fn get_model_settings(&self, session_id: &str) -> anyhow::Result<std::collections::HashMap<String, String>> {
        let rows = sqlx::query(
            "SELECT task_type, model_name FROM model_settings WHERE session_id = ?"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut settings = std::collections::HashMap::new();
        for row in rows {
            settings.insert(
                row.get::<String, _>("task_type"),
                row.get::<String, _>("model_name")
            );
        }

        Ok(settings)
    }

    /// Save model settings for a session
    pub async fn save_model_settings(
        &self,
        session_id: &str,
        settings: &std::collections::HashMap<String, String>
    ) -> anyhow::Result<()> {
        for (task_type, model_name) in settings {
            sqlx::query(
                "INSERT INTO model_settings (session_id, task_type, model_name, updated_at)
                 VALUES (?, ?, ?, CURRENT_TIMESTAMP)
                 ON CONFLICT(session_id, task_type)
                 DO UPDATE SET model_name = ?, updated_at = CURRENT_TIMESTAMP"
            )
            .bind(session_id)
            .bind(task_type)
            .bind(model_name)
            .bind(model_name)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get model for a specific task type
    pub async fn get_model_for_task(
        &self,
        session_id: &str,
        task_type: &str
    ) -> anyhow::Result<Option<String>> {
        let row = sqlx::query(
            "SELECT model_name FROM model_settings WHERE session_id = ? AND task_type = ?"
        )
        .bind(session_id)
        .bind(task_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("model_name")))
    }

    /// Save API key for a provider (encrypted)
    pub async fn save_api_key(
        &self,
        session_id: &str,
        provider: &str,
        encrypted_key: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO api_keys (session_id, provider, encrypted_key, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(session_id, provider)
             DO UPDATE SET encrypted_key = ?, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(session_id)
        .bind(provider)
        .bind(encrypted_key)
        .bind(encrypted_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get API key for a specific provider (returns encrypted value)
    pub async fn get_api_key(
        &self,
        session_id: &str,
        provider: &str,
    ) -> anyhow::Result<Option<String>> {
        let row = sqlx::query(
            "SELECT encrypted_key FROM api_keys WHERE session_id = ? AND provider = ?"
        )
        .bind(session_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("encrypted_key")))
    }

    /// Get all API keys for a session (returns HashMap of provider -> encrypted_key)
    pub async fn get_all_api_keys(
        &self,
        session_id: &str,
    ) -> anyhow::Result<std::collections::HashMap<String, String>> {
        let rows = sqlx::query(
            "SELECT provider, encrypted_key FROM api_keys WHERE session_id = ?"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut keys = std::collections::HashMap::new();
        for row in rows {
            keys.insert(
                row.get::<String, _>("provider"),
                row.get::<String, _>("encrypted_key"),
            );
        }

        Ok(keys)
    }

    /// Delete API key for a specific provider
    pub async fn delete_api_key(
        &self,
        session_id: &str,
        provider: &str,
    ) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE session_id = ? AND provider = ?")
            .bind(session_id)
            .bind(provider)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete all API keys for a session
    pub async fn delete_all_api_keys(&self, session_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Remote connection operations

    /// Save a remote connection
    pub async fn save_remote_connection(
        &self,
        session_id: &str,
        connection_id: &str,
        host: &str,
        port: u16,
        username: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO remote_connections (id, session_id, host, port, username, last_connected)
             VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET last_connected = CURRENT_TIMESTAMP"
        )
        .bind(connection_id)
        .bind(session_id)
        .bind(host)
        .bind(port as i64)
        .bind(username)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get remote connections for a session
    pub async fn get_remote_connections(&self, session_id: &str) -> anyhow::Result<Vec<RemoteConnectionInfo>> {
        let rows = sqlx::query(
            "SELECT id, host, port, username, created_at, last_connected
             FROM remote_connections
             WHERE session_id = ?
             ORDER BY last_connected DESC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| RemoteConnectionInfo {
                id: row.get("id"),
                host: row.get("host"),
                port: row.get::<i64, _>("port") as u16,
                username: row.get("username"),
                status: "disconnected".to_string(), // Status is determined at runtime
            })
            .collect())
    }

    /// Delete a remote connection
    pub async fn delete_remote_connection(&self, connection_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM remote_connections WHERE id = ?")
            .bind(connection_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub snapshot_id: String,
    pub execution_id: String,
    pub node_id: String,
    pub node_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConnectionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_initialization() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Check if database was created successfully
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
            .fetch_one(db.pool())
            .await
            .expect("Failed to count tables");

        // Should have multiple tables created
        assert!(count.0 > 0);
    }

    #[tokio::test]
    async fn test_virtual_office_space_creation() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Check if Main Office was created
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM virtual_office_spaces WHERE name = 'Main Office'"
        )
        .fetch_one(db.pool())
        .await
        .expect("Failed to check space");

        assert_eq!(count.0, 1, "Main Office should be created");
    }

    #[tokio::test]
    async fn test_virtual_office_objects_creation() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Check if objects were created
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM virtual_office_objects")
            .fetch_one(db.pool())
            .await
            .expect("Failed to count objects");

        // Should have meeting rooms, desks, etc.
        assert!(count.0 > 0, "Virtual office objects should be created");
    }

    #[tokio::test]
    async fn test_slack_channel_creation() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Check if default channels were created
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM berrychat_channels")
            .fetch_one(db.pool())
            .await
            .expect("Failed to count channels");

        assert!(count.0 >= 3, "Default BerryChat channels should be created");
    }

    #[tokio::test]
    async fn test_workflow_table_exists() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Check if workflow table exists
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='workflow_executions'"
        )
        .fetch_one(db.pool())
        .await
        .expect("Failed to check table");

        assert_eq!(count.0, 1, "workflow_executions table should exist");
    }

    #[tokio::test]
    async fn test_insert_virtual_office_user() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Get the default space
        let space_id: (String,) = sqlx::query_as("SELECT id FROM virtual_office_spaces LIMIT 1")
            .fetch_one(db.pool())
            .await
            .expect("Failed to get space");

        // Insert a test user
        let user_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO virtual_office_users (id, space_id, user_id, username, x, y, direction, avatar, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&user_id)
        .bind(&space_id.0)
        .bind("test_user")
        .bind("TestUser")
        .bind(25)
        .bind(25)
        .bind("down")
        .bind("👤")
        .bind("online")
        .execute(db.pool())
        .await
        .expect("Failed to insert user");

        // Verify user was inserted
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM virtual_office_users WHERE id = ?")
            .bind(&user_id)
            .fetch_one(db.pool())
            .await
            .expect("Failed to count users");

        assert_eq!(count.0, 1, "User should be inserted");
    }

    #[tokio::test]
    async fn test_insert_slack_message() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Get a default channel
        let channel_id: (String,) = sqlx::query_as("SELECT id FROM berrychat_channels LIMIT 1")
            .fetch_one(db.pool())
            .await
            .expect("Failed to get channel");

        // Insert a test message
        let message_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO berrychat_messages (id, channel_id, user_id, username, content, timestamp)
             VALUES (?, ?, ?, ?, ?, datetime('now'))"
        )
        .bind(&message_id)
        .bind(&channel_id.0)
        .bind("test_user")
        .bind("TestUser")
        .bind("Hello, World!")
        .execute(db.pool())
        .await
        .expect("Failed to insert message");

        // Verify message was inserted
        let content: (String,) = sqlx::query_as("SELECT content FROM berrychat_messages WHERE id = ?")
            .bind(&message_id)
            .fetch_one(db.pool())
            .await
            .expect("Failed to fetch message");

        assert_eq!(content.0, "Hello, World!", "Message content should match");
    }

    #[tokio::test]
    async fn test_virtual_office_object_walkability() {
        let db = Database::new("sqlite::memory:").await.expect("Failed to create database");

        // Check wall objects (should be non-walkable)
        let walls: Vec<(i32,)> = sqlx::query_as(
            "SELECT walkable FROM virtual_office_objects WHERE object_type = 'wall'"
        )
        .fetch_all(db.pool())
        .await
        .expect("Failed to fetch walls");

        // All walls should be non-walkable (walkable = 0)
        for wall in walls {
            assert_eq!(wall.0, 0, "Walls should not be walkable");
        }

        // Check room objects (should be walkable)
        let rooms: Vec<(i32,)> = sqlx::query_as(
            "SELECT walkable FROM virtual_office_objects WHERE object_type = 'room'"
        )
        .fetch_all(db.pool())
        .await
        .expect("Failed to fetch rooms");

        // All rooms should be walkable (walkable = 1)
        for room in rooms {
            assert_eq!(room.0, 1, "Rooms should be walkable");
        }
    }
}
