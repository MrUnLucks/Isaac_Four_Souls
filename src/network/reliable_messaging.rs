use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use uuid::Uuid;

// Global sequence counter - simple and thread-safe
static GLOBAL_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliableMessage {
    pub id: String,
    pub sequence: u64,
    pub payload: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAck {
    pub message_id: String,
}

pub fn create_reliable_message(payload: String) -> ReliableMessage {
    ReliableMessage {
        id: Uuid::new_v4().to_string(),
        sequence: GLOBAL_SEQUENCE.fetch_add(1, Ordering::SeqCst),
        payload,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    }
}

#[derive(Debug)]
pub struct PendingMessage {
    pub message: ReliableMessage,
    pub send_time: Instant,
    pub retry_count: u32,
}

pub struct MessageReceiver {
    expected_sequence: u64,
    message_buffer: HashMap<u64, ReliableMessage>,
}

impl MessageReceiver {
    pub fn new() -> Self {
        Self {
            expected_sequence: 1,
            message_buffer: HashMap::new(),
        }
    }

    // Returns: (ack_to_send, ordered_messages_to_process)
    pub fn receive_message(
        &mut self,
        message: ReliableMessage,
    ) -> (MessageAck, Vec<ReliableMessage>) {
        let ack = MessageAck {
            message_id: message.id.clone(),
        };

        // Always ack, but check for processing
        if message.sequence < self.expected_sequence {
            // Old message - already processed
            return (ack, vec![]);
        }

        if message.sequence == self.expected_sequence {
            // Process this and any buffered consecutive ones
            let mut to_process = vec![message];
            self.expected_sequence += 1;

            // Drain consecutive buffered messages
            while let Some(buffered) = self.message_buffer.remove(&self.expected_sequence) {
                to_process.push(buffered);
                self.expected_sequence += 1;
            }

            (ack, to_process)
        } else {
            // Future message - buffer it
            self.message_buffer.insert(message.sequence, message);
            (ack, vec![])
        }
    }
}
