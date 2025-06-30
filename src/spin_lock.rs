use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

unsafe impl<T> Sync for SpinLock<T> where T: Send {}
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}
// 'a to ensure guard's lifetime is shorter than lock
pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}
impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> Guard<T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        Guard { lock: self }
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let x = SpinLock::new(Vec::new());

        std::thread::scope(|s| {
            s.spawn(|| x.lock().push(1));
            s.spawn(|| {
                let mut v = x.lock();
                v.push(2);
                v.push(3);
            });
        });

        let g = x.lock();
        assert!(g.as_slice() == [1, 2, 3] || g.as_slice() == [2, 3, 1]);
    }
}
