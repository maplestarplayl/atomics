use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU64, Ordering},
};

// A struct to ensure cache line alignment to prevent **false sharing**.
#[repr(align(64))]
struct CachePadded<T>(pub T);

///FIFO2:
///  - Use atomic operations to manage head and tail indices.
///  - Use UnsafeCell to solve interior mutability issues.
///  - Use `Ordering::SeqCst` to ensure strong memory ordering guarantees.
///FIFO3:
///  - Use `CachePadded` struct to wrap atomic variables, preventing false sharing.
///  - Use `Ordering::Acquire` and `Ordering::Release` for better performance while maintaining correctness.
/// 
/// 
/// Guidelines for using atomic orderings
/// 1. Use `Ordering::SeqCst` for simplicity and strong guarantees.
/// 2. Use `Ordering::Acquire` for loads that other threads write to.
/// 3. Use `Ordering::Release` for stores that other threads read from.
/// 4. Use `Ordering::AcqRel` for read-modify-write operations.
/// 5. Use `Ordering::Relaxed` for operations that don't require ordering guarantees.
pub struct Fifo3<T> {
    buffer: Vec<UnsafeCell<MaybeUninit<T>>>,
    capacity: usize,
    head: CachePadded<AtomicU64>,
    tail: CachePadded<AtomicU64>,
}

unsafe impl<T: Send> Send for Fifo3<T> {}
unsafe impl<T: Send> Sync for Fifo3<T> {}

impl<T: Send> Fifo3<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);

        let mut buffer = Vec::with_capacity(capacity);

        unsafe {
            buffer.set_len(capacity);
        }

        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            buffer,
            capacity,
            head: CachePadded(AtomicU64::new(0)),
            tail: CachePadded(AtomicU64::new(0)),
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        let head = self.head.0.load(Ordering::Relaxed);
        // Use `Relaxed` since not need to sync with other threads here.
        // Only need an approximate value of tail to check for full queue.
        let tail = self.tail.0.load(Ordering::Relaxed);

        if head.wrapping_sub(tail) == self.capacity as u64 {
            return Err(value);
        }

        let index = head % self.capacity as u64;

        unsafe {
            (*self.buffer.get_unchecked_mut(index as usize).get()).write(value);
        }

        self.head.0.store(head + 1, Ordering::Release);

        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        let head = self.head.0.load(Ordering::Acquire);
        let tail = self.tail.0.load(Ordering::Relaxed);

        if head == tail {
            return None;
        }

        let index = tail % self.capacity as u64;

        let value =
            unsafe { (*self.buffer.get_unchecked(index as usize).get()).assume_init_read() };

        //Since the producer use `Relaxed` to load head, here we use `Relaxed` to load tail.
        self.tail.0.store(tail + 1, Ordering::Relaxed);

        Some(value)
    }

    /// 返回队列的容量。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 检查队列是否为空。
    pub fn is_empty(&self) -> bool {
        // 使用 Acquire 确保我们能看到最新的 push 值
        self.head.0.load(Ordering::Acquire) == self.tail.0.load(Ordering::Acquire)
    }
}

/// 实现 Drop trait 以确保队列销毁时，其中剩余的元素能够被正确地丢弃。
impl<T> Drop for Fifo3<T> {
    fn drop(&mut self) {
        // 在 drop 中我们拥有 &mut self，可以直接访问内部数据。
        let push_cursor = *self.head.0.get_mut();
        let mut pop_cursor = *self.tail.0.get_mut();

        while pop_cursor < push_cursor {
            let index = (pop_cursor % self.capacity as u64) as usize;
            unsafe {
                // 安全地读取值的所有权，然后让它在离开作用域时被 drop。
                (*self.buffer.get_unchecked(index).get()).assume_init_read();
            }
            pop_cursor += 1;
        }
    }
}
