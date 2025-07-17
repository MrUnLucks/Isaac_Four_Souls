use futures_util::{stream::SplitSink, SinkExt};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
// Represents a single WebSocket connection
#[derive(Debug)]
struct WebSocketConnection {
    id: String,
    sender: SplitSink<WebSocketStream<TcpStream>, Message>,
}
pub struct ConnectionManager {
    connections: HashMap<String, WebSocketConnection>,
}
impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn add_connection(
        &mut self,
        id: String,
        sender: SplitSink<WebSocketStream<TcpStream>, Message>,
    ) {
        let connection = WebSocketConnection {
            id: id.clone(),
            sender,
        };
        self.connections.insert(id.clone(), connection);
        println!("📝 Added connection: {}", id);
    }

    pub fn remove_connection(&mut self, id: &str) {
        self.connections.remove(id);
        println!("🗑️ Removed connection: {}", id);
    }

    pub async fn send_to_all(&mut self, message: &str) {
        println!("📢 Broadcasting: {}", message);

        let mut failed_connections = Vec::new();

        for (id, connection) in &mut self.connections {
            if let Err(e) = connection
                .sender
                .send(Message::Text(message.to_string()))
                .await
            {
                eprintln!("❌ Failed to send to connection {}: {}", id, e);
                failed_connections.push(id.clone());
            }
        }

        // Remove failed connections
        for failed_id in failed_connections {
            self.remove_connection(&failed_id);
        }
    }

    pub async fn send_to_player(
        &mut self,
        connection_id: &str,
        message: &str,
    ) -> Result<(), String> {
        self.connections
            .get_mut(connection_id)
            .ok_or_else(|| "Connection not found".to_string())?
            .sender
            .send(Message::Text(message.to_string()))
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;
        Ok(())
    }
}
