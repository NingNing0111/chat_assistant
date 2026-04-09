use rig::message::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session manager for multi-turn conversations
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    current_session_id: Arc<RwLock<Option<String>>>,
    max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub persona: String,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    max_history: usize,
}

impl Session {
    pub fn new(persona: &str, max_history: usize) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            persona: persona.into(),
            messages: Vec::new(),
            created_at: now,
            last_active: now,
            max_history,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let msg = match role {
            "user" => Message::user(content),
            "assistant" => Message::assistant(content),
            _ => Message::user(content),
        };
        self.messages.push(msg);
        self.last_active = chrono::Utc::now();
    }

    pub fn get_context(&self) -> Vec<Message> {
        let len = self.messages.len();
        if len <= self.max_history {
            self.messages.clone()
        } else {
            // Keep the most recent messages
            self.messages[len - self.max_history..].to_vec()
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.last_active = chrono::Utc::now();
    }
}

impl SessionManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            current_session_id: Arc::new(RwLock::new(None)),
            max_history,
        }
    }

    /// Create a new session
    pub async fn create_session(&self, persona: &str) -> String {
        let session = Session::new(persona, self.max_history);
        let id = session.id.clone();
        self.sessions.write().await.insert(id.clone(), session);
        *self.current_session_id.write().await = Some(id.clone());
        id
    }

    /// Get or create the current session
    pub async fn get_or_create_session(&self, persona: &str) -> String {
        if let Some(id) = self.current_session_id.read().await.clone() {
            if self.sessions.read().await.contains_key(&id) {
                return id;
            }
        }
        self.create_session(persona).await
    }

    /// Switch to a specific session
    pub async fn switch_session(&self, session_id: &str) -> bool {
        if self.sessions.read().await.contains_key(session_id) {
            *self.current_session_id.write().await = Some(session_id.to_string());
            true
        } else {
            false
        }
    }

    /// Add a message to the current session
    pub async fn add_message(&self, role: &str, content: &str) {
        if let Some(id) = self.current_session_id.read().await.clone() {
            if let Some(session) = self.sessions.write().await.get_mut(&id) {
                session.add_message(role, content);
            }
        }
    }

    /// Get messages from current session
    pub async fn get_messages(&self) -> Vec<Message> {
        if let Some(id) = self.current_session_id.read().await.clone() {
            if let Some(session) = self.sessions.read().await.get(&id) {
                return session.get_context();
            }
        }
        vec![]
    }

    /// Clear current session
    pub async fn clear_current(&self) {
        if let Some(id) = self.current_session_id.read().await.clone() {
            if let Some(session) = self.sessions.write().await.get_mut(&id) {
                session.clear();
            }
        }
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
        if let Some(current) = self.current_session_id.read().await.clone() {
            if current == session_id {
                *self.current_session_id.write().await = None;
            }
        }
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<(String, String, chrono::DateTime<chrono::Utc>)> {
        self.sessions
            .read()
            .await
            .iter()
            .map(|(id, s)| (id.clone(), s.persona.clone(), s.last_active))
            .collect()
    }

    /// Get current session ID
    pub async fn current_session_id(&self) -> Option<String> {
        self.current_session_id.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session() {
        let manager = SessionManager::new(10);
        let id = manager.create_session("assistant").await;

        manager.add_message("user", "Hello").await;
        manager.add_message("assistant", "Hi there!").await;

        let messages = manager.get_messages().await;
        assert_eq!(messages.len(), 2);

        manager.clear_current().await;
        let messages = manager.get_messages().await;
        assert_eq!(messages.len(), 0);
    }
}
