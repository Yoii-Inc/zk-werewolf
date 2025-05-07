use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatLog {
    pub room_id: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: String,
    pub player_id: String,
    pub player_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: ChatMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatMessageType {
    Public,  // 全体チャット（昼）
    Wolf,    // 人狼チャット（夜）
    Private, // プライベートメッセージ（占い結果など）
    System,  // システムメッセージ
}

impl ChatLog {
    pub fn new(room_id: String) -> Self {
        ChatLog {
            room_id,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn add_system_message(&mut self, content: String) {
        let system_message = ChatMessage::new(
            "system".to_string(),
            "System".to_string(),
            content,
            ChatMessageType::System,
        );
        self.add_message(system_message);
    }

    pub fn get_messages_by_type(&self, message_type: ChatMessageType) -> Vec<&ChatMessage> {
        self.messages
            .iter()
            .filter(|m| m.message_type == message_type)
            .collect()
    }
}

impl ChatMessage {
    pub fn new(
        player_id: String,
        player_name: String,
        content: String,
        message_type: ChatMessageType,
    ) -> Self {
        ChatMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            player_id,
            player_name,
            content,
            timestamp: Utc::now(),
            message_type,
        }
    }
}
