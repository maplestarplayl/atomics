use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
    in_use: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}
unsafe impl<T> Send for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
            in_use: AtomicBool::new(false),
        }
    }

    pub unsafe fn send(&self, message: T) {
        if !self.in_use.swap(true, Ordering::Acquire) {
            panic!("can't send more than once")
        }
        unsafe { (*self.message.get()).write(message) };

        self.ready.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub unsafe fn receive(&self) -> T {
        if !self.is_ready() {
            panic!("no message available");
        }
        if !self.ready.swap(false, Ordering::Acquire) {
            panic!("no message available")
        }
        unsafe { (*self.message.get()).assume_init_read() }
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
    use std::thread;

    use super::*;

    // #[test]
    // fn one_shot_channel_basic() {
    //     let ch = Channel::new();
    //     let t = thread::current();
    //     thread::scope(|s| {
    //         s.spawn(|| {
    //             unsafe { ch.send("a") };
    //             t.unpark();
    //         });
            
    //         while !ch.is_ready() {
    //             thread::park();
    //         }

    //         assert_eq!(unsafe { ch.receive() }, "a")
    //     });
    // }
}
