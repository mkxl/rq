use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct Channel<T> {
    pub receiver: UnboundedReceiver<T>,
    pub sender: UnboundedSender<T>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self { receiver, sender }
    }
}
