use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory store for short-term and long-term memory
pub struct MemoryStore {
    short_term: Arc<RwLock<ShortTermMemory>>,
    long_term: Arc<RwLock<LongTermMemory>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShortTermMemory {
    /// Current conversation context
    pub context: Vec<MemoryEntry>,
    /// User preferences in current session
    pub preferences: HashMap<String, String>,
    /// Recent entities mentioned
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LongTermMemory {
    /// Persistent user preferences
    pub user_preferences: HashMap<String, String>,
    /// Summary of past conversations
    pub conversation_summaries: Vec<ConversationSummary>,
    /// User profile data
    pub user_profile: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub session_id: String,
    pub summary: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    pub name: Option<String>,
    pub language: String,
    pub timezone: String,
    pub interests: Vec<String>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            short_term: Arc::new(RwLock::new(ShortTermMemory::default())),
            long_term: Arc::new(RwLock::new(LongTermMemory::default())),
        }
    }

    /// Add a memory entry to short-term memory
    pub async fn add_short_term(&self, role: &str, content: &str) {
        let mut memory = self.short_term.write().await;
        memory.context.push(MemoryEntry {
            role: role.into(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
        });
    }

    /// Get short-term memory context
    pub async fn get_short_term(&self) -> Vec<MemoryEntry> {
        self.short_term.read().await.context.clone()
    }

    /// Clear short-term memory
    pub async fn clear_short_term(&self) {
        self.short_term.write().await.context.clear();
    }

    /// Save user preference
    pub async fn save_preference(&self, key: &str, value: &str) {
        let mut memory = self.short_term.write().await;
        memory.preferences.insert(key.into(), value.into());

        // Also persist to long-term
        let mut long_term = self.long_term.write().await;
        long_term.user_preferences.insert(key.into(), value.into());
    }

    /// Get user preference
    pub async fn get_preference(&self, key: &str) -> Option<String> {
        // Check short-term first
        if let Some(val) = self.short_term.read().await.preferences.get(key) {
            return Some(val.clone());
        }
        // Then long-term
        self.long_term.read().await.user_preferences.get(key).cloned()
    }

    /// Add conversation summary to long-term memory
    pub async fn add_summary(&self, session_id: &str, summary: &str, topics: Vec<String>) {
        let mut long_term = self.long_term.write().await;
        long_term.conversation_summaries.push(ConversationSummary {
            session_id: session_id.into(),
            summary: summary.into(),
            timestamp: chrono::Utc::now(),
            topics,
        });

        // Keep only last 100 summaries
        if long_term.conversation_summaries.len() > 100 {
            long_term.conversation_summaries.remove(0);
        }
    }

    /// Get recent conversation summaries
    pub async fn get_recent_summaries(&self, limit: usize) -> Vec<ConversationSummary> {
        let long_term = self.long_term.read().await;
        let len = long_term.conversation_summaries.len();
        if len <= limit {
            long_term.conversation_summaries.clone()
        } else {
            long_term.conversation_summaries[len - limit..].to_vec()
        }
    }

    /// Update user profile
    pub async fn update_profile(&self, profile: UserProfile) {
        self.long_term.write().await.user_profile = profile;
    }

    /// Get user profile
    pub async fn get_profile(&self) -> UserProfile {
        self.long_term.read().await.user_profile.clone()
    }

    /// Format memory as context string for LLM
    pub async fn format_context(&self) -> String {
        let short_term = self.short_term.read().await;

        let mut context = String::new();

        if !short_term.context.is_empty() {
            context.push_str("## Recent Conversation\n");
            for entry in &short_term.context {
                context.push_str(&format!("{}: {}\n", entry.role, entry.content));
            }
        }

        let long_term = self.long_term.read().await;
        if !long_term.user_preferences.is_empty() {
            context.push_str("\n## User Preferences\n");
            for (k, v) in &long_term.user_preferences {
                context.push_str(&format!("{}: {}\n", k, v));
            }
        }

        context
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
