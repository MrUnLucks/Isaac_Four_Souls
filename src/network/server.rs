use crate::actors::actor_registry::ActorRegistry;
use crate::actors::lobby_actor::{LobbyActor, LobbyMessage};
use std::{error::Error, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{mpsc, Mutex},
};
use uuid::Uuid;

use crate::{
    CommandProcessor, ConnectionCommand, ConnectionHandler, ConnectionManager,
    GameMessageLoopRegistry, RoomManager,
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
        let mut connection_manager = ConnectionManager::new();

        // Create channel for connection management commands
        let (cmd_sender, mut cmd_receiver) = mpsc::unbounded_channel::<ConnectionCommand>();

        tokio::spawn(async move {
            while let Some(command) = cmd_receiver.recv().await {
                let processed_command =
                    CommandProcessor::process_command(command, &mut connection_manager).await;
                if processed_command.is_err() {
                    return;
                }
            }
        });

        let game_message_loop_registry = Arc::new(GameMessageLoopRegistry::new());

        let (lobby_sender, lobby_receiver) = mpsc::unbounded_channel::<LobbyMessage>();
        let mut lobby_actor =
            LobbyActor::new(game_message_loop_registry.clone(), cmd_sender.clone());

        tokio::spawn(async move {
            lobby_actor.run(lobby_receiver).await;
        });

        let actor_registry = Arc::new(ActorRegistry::new(lobby_sender));

        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 New connection from: {}", addr);
            let connection_id = Uuid::new_v4().to_string();

            let actor_registry = actor_registry.clone();
            let cmd_sender = cmd_sender.clone();
            let game_registry = game_message_loop_registry.clone();

            tokio::spawn(async move {
                if let Err(e) = ConnectionHandler::handle_connection(
                    stream,
                    connection_id,
                    actor_registry,
                    cmd_sender,
                    game_registry,
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
