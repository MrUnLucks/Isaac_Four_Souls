pub trait Messageable {
    fn send_message(&self, message: String);
    fn get_id(&self) -> &str;
}

fn broadcast_to_all<T: Messageable>(recipients: &[T], message: String) {
    for recipient in recipients {
        recipient.send_message(message.clone());
    }
}
