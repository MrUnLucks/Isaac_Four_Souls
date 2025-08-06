use std::{error::Error, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{mpsc, Mutex},
};
use uuid::Uuid;

use crate::network::lobby::LobbyState;
use crate::network::websocket::{
    commands::{CommandProcessor, ConnectionCommand},
    connection::ConnectionHandler,
};

pub struct WebsocketServer {
    address: String,
}

impl WebsocketServer {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(&self.address).await?;
        let lobby_state = Arc::new(Mutex::new(LobbyState::new()));

        // Create channel for connection management commands
        let (cmd_sender, mut cmd_receiver) = mpsc::unbounded_channel::<ConnectionCommand>();

        // Spawn connection manager task
        let lobby_state_clone = lobby_state.clone();
        tokio::spawn(async move {
            while let Some(command) = cmd_receiver.recv().await {
                let mut state = lobby_state_clone.lock().await;
                CommandProcessor::process_command(command, &mut state).await;
            }
        });

        // Accept connections
        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 New connection from: {}", addr);
            let connection_id = Uuid::new_v4().to_string();

            let lobby_state = lobby_state.clone();
            let cmd_sender = cmd_sender.clone();

            tokio::spawn(async move {
                if let Err(e) = ConnectionHandler::handle_connection(
                    stream,
                    connection_id,
                    lobby_state,
                    cmd_sender,
                )
                .await
                {
                    eprintln!("❌ Error handling connection: {}", e);
                }
            });
        }

        Ok(())
    }
}
