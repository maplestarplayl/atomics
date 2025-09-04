use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::{AtomicU64, Ordering}};


///Fifo2:
///  - Use atomic operations to manage head and tail indices.
///  - Use UnsafeCell to solve interior mutability issues.
///  - Use `Ordering::SeqCst` to ensure strong memory ordering guarantees.
pub struct Fifo2<T> {
    buffer: Vec<UnsafeCell<MaybeUninit<T>>>,
    capacity: usize,
    head: AtomicU64,
    tail: AtomicU64,
}

unsafe impl <T: Send> Send for Fifo2<T> {}
unsafe impl <T: Send> Sync for Fifo2<T> {}


impl<T: Send> Fifo2<T> {
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
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        if head.wrapping_sub(tail) == self.capacity as u64 {
            return Err(value);
        }

        let index = head % self.capacity as u64;

        unsafe {
            (*self.buffer.get_unchecked_mut(index as usize).get()).write(value);
        }

        self.head.store(head + 1, Ordering::SeqCst);

        Ok(())
    }


    pub fn pop(&mut self) -> Option<T> {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        if head == tail {
            return None;
        }

        let index = tail % self.capacity as u64;

        let value = unsafe {
            (*self.buffer.get_unchecked(index as usize).get()).assume_init_read()
        };

        self.tail.store(tail + 1, Ordering::SeqCst);
        Some(value)
    }

    /// 返回队列的容量。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 返回队列中当前的元素数量 (可能不是最新的)。
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        (head.wrapping_sub(tail)) as usize
    }

    /// 检查队列是否为空。
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::SeqCst) == self.tail.load(Ordering::SeqCst)
    }

    /// 检查队列是否已满。
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        head.wrapping_sub(tail) as usize == self.capacity
    }
}

/// 实现 Drop trait 以确保队列销毁时，其中剩余的元素能够被正确地丢弃。
impl<T> Drop for Fifo2<T> {
    fn drop(&mut self) {
        // 在 drop 中我们拥有 &mut self，可以直接访问内部数据。
        let push_cursor = *self.head.get_mut();
        let mut pop_cursor = *self.tail.get_mut();

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

