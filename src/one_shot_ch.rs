use std::cell::UnsafeCell;
use std::iter::Rev;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
//     let ch = Arc::new(Channel {
//         message: UnsafeCell::new(MaybeUninit::uninit()),
//         ready: AtomicBool::new(false),
//     });
//     (
//         Sender {
//             channel: ch.clone(),
//         },
//         Receiver { channel: ch },
//     )
// }

impl<T> Channel<T> {
    pub fn new() -> Self {
        Channel {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        *self = Self::new();
        (Sender { channel: self }, Receiver { channel: self })
    }
}
unsafe impl<T> Sync for Channel<T> where T: Send {}
pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
}

impl<T> Sender<'_, T> {
    pub fn send(self, message: T) {
        unsafe {
            (*self.channel.message.get()).write(message);
        }
        self.channel.ready.store(true, Ordering::Release);
    }
}

impl<T> Receiver<'_, T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Ordering::Relaxed)
    }
    pub fn receive(self) -> T {
        if !self.channel.ready.swap(false, Ordering::Acquire) {
            panic!("no available message");
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn main() {
        let mut ch = Channel::new();
        thread::scope(|s| {
            let (sender, receiver) = ch.split();
            let t = thread::current();
            s.spawn(move || {
                sender.send("a");
                t.unpark();
            });
            while !receiver.is_ready() {
                thread::park();
            }
            assert_eq!(receiver.receive(), "a");
        });
    }
}
