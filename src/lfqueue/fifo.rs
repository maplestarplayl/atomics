use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
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
///FIFO4:
///  - Use local cache variables in Producer and Consumer to reduce the number of atomic loads.
///  - Split `Fifo` into `Producer` and `Consumer` structs for better separation of concerns.
///
///
/// Guidelines for using atomic orderings
/// 1. Use `Ordering::SeqCst` for simplicity and strong guarantees.
/// 2. Use `Ordering::Acquire` for loads that other threads write to.
/// 3. Use `Ordering::Release` for stores that other threads read from.
/// 4. Use `Ordering::AcqRel` for read-modify-write operations.
/// 5. Use `Ordering::Relaxed` for operations that don't require ordering guarantees.
struct Shared<T: Send> {
    buffer: Vec<UnsafeCell<MaybeUninit<T>>>,
    capacity: usize,
    head: CachePadded<AtomicU64>,
    tail: CachePadded<AtomicU64>,
}

pub struct Producer<T: Send> {
    shared: Arc<Shared<T>>,
    head: u64,
    // Local cache of producer's tail to reduce atomic loads
    cache_tail: u64,
}

pub struct Consumer<T: Send> {
    shared: Arc<Shared<T>>,
    tail: u64,
    // Local cache of consumer's head to reduce atomic loads
    cache_head: u64,
}

pub fn new<T: Send>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    assert!(capacity > 0);

    let mut buffer = Vec::with_capacity(capacity);
    for _ in 0..capacity {
        buffer.push(UnsafeCell::new(MaybeUninit::uninit()));
    }

    unsafe {
        buffer.set_len(capacity);
    }
    let shared = Arc::new(Shared {
        buffer,
        capacity,
        head: CachePadded(AtomicU64::new(0)),
        tail: CachePadded(AtomicU64::new(0)),
    });
    let producer = Producer {
        shared: shared.clone(),
        head: 0,
        cache_tail: 0,
    };
    let consumer = Consumer {
        shared,
        tail: 0,
        cache_head: 0,
    };

    (producer, consumer)
}

impl<T: Send> Producer<T> {
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.head - self.cache_tail == self.shared.capacity as u64 {
            // Update local cache of tail
            self.cache_tail = self.shared.tail.0.load(Ordering::Acquire);

            if self.head - self.cache_tail == self.shared.capacity as u64 {
                return Err(value);
            }
        }

        let index = self.head % self.shared.capacity as u64;

        unsafe {
            (*self.shared.buffer[index as usize].get())
                .as_mut_ptr()
                .write(value);
        }

        self.head += 1;

        self.shared.head.0.store(self.head, Ordering::Release);
        Ok(())
    }
}

impl<T: Send> Consumer<T> {
    pub fn pop(&mut self) -> Option<T> {
        if self.tail == self.cache_head {
            // Update local cache of head
            self.cache_head = self.shared.head.0.load(Ordering::Acquire);

            if self.tail == self.cache_head {
                return None;
            }
        }

        let index = self.tail % self.shared.capacity as u64;

        let value = unsafe { (*self.shared.buffer[index as usize].get()).as_ptr().read() };

        self.tail += 1;

        self.shared.tail.0.store(self.tail, Ordering::Release);
        Some(value)
    }
}

impl<T: Send> Drop for Consumer<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
