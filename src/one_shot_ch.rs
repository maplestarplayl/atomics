use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    pub unsafe fn send(&self, message: T) {
        unsafe { (*self.message.get()).write(message) };

        self.ready.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub unsafe fn receive(&self) -> T {
        unsafe { (*self.message.get()).assume_init_read() }
    }

}

