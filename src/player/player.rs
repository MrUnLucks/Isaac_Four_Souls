use std::fmt;
use uuid::Uuid;

use crate::player::traits::Messageable;

#[derive(Debug)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub is_connected: bool,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {} ({}), is_connected: {}",
            self.name, self.id, self.is_connected
        )
    }
}
impl Player {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            is_connected: true,
        }
    }
    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }
}
impl Messageable for Player {
    fn get_id(&self) -> &str {
        &self.id
    }
    fn send_message(&self, message: String) {
        println!("Sending to {}: {}", self.name, message);
    }
}
