use std::{collections::VecDeque, sync::{Condvar, Mutex}};
pub struct Channel<T> {
    queue: Mutex<VecDeque<T>>,
    item_ready: Condvar,
}
// This implementation is simple and easy to use
// But its effiency is pretty low since any
// send or recv operation will block other operations for a while
impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            item_ready: Condvar::new(),
        }
    }

    pub fn send(&self, message: T) {
        self.queue.lock().unwrap().push_back(message);
        self.item_ready.notify_one();
    }

    pub fn receive(&self) -> T {
        let mut queue = self.queue.lock().unwrap();
        loop {
            if let Some(message) = queue.pop_back() {
                return message;
            }
            queue = self.item_ready.wait(queue).unwrap()
        }
    }
}