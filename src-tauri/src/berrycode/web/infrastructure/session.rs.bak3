//! Session management for web version

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_root: PathBuf,
    pub files: Vec<PathBuf>,
    pub chat_history: Vec<ChatMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub shared: bool,
    pub share_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn new(project_root: PathBuf) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_root,
            files: Vec::new(),
            chat_history: Vec::new(),
            created_at: now,
            last_activity: now,
            shared: false,
            share_url: None,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
    }

    pub fn add_message(&mut self, role: String, content: String) {
        self.chat_history.push(ChatMessage {
            role,
            content,
            timestamp: chrono::Utc::now(),
        });
        self.update_activity();
    }

    pub fn add_file(&mut self, file: PathBuf) {
        if !self.files.contains(&file) {
            self.files.push(file);
            self.update_activity();
        }
    }

    pub fn remove_file(&mut self, file: &PathBuf) {
        self.files.retain(|f| f != file);
        self.update_activity();
    }

    pub fn enable_sharing(&mut self) -> String {
        self.shared = true;
        let share_url = format!("/share/{}", self.id);
        self.share_url = Some(share_url.clone());
        share_url
    }
}

/// Session store
#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_session(&self, project_root: PathBuf) -> String {
        let session = Session::new(project_root);
        let session_id = session.id.clone();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.clone(), session);

        session_id
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    pub fn update_session<F>(&self, session_id: &str, f: F) -> bool
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            f(session);
            true
        } else {
            false
        }
    }

    pub fn delete_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(session_id).is_some()
    }

    pub fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let mut sessions = self.sessions.write().unwrap();
        let now = chrono::Utc::now();

        sessions.retain(|_, session| {
            let age = now.signed_duration_since(session.last_activity);
            age.num_hours() < max_age_hours
        });
    }

    pub fn list_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().unwrap();
        sessions.values().cloned().collect()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
