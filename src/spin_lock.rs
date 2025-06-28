use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

unsafe impl<T> Sync for SpinLock<T> where T: Send {}
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value)
        }
    }

    pub fn lock(&self) -> &mut T{
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        unsafe {&mut *self.value.get()}
    }

    pub unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
