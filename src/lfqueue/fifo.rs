use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ops::Deref,
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
///FIFO5:
///  - Introduce `Pusher` and `Popper` proxy objects to implement `zero copy`
///  - Use `Drop` trait to automatically handle head/tail updates when the proxy objects go out of scope.
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

unsafe impl<T: Send> Send for Producer<T> {}
unsafe impl<T: Send> Send for Consumer<T> {}

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
// --- Fifo5: Proxy Objects ---

pub struct Pusher<'a, T: Send> {
    producer: &'a mut Producer<T>,
    slot: *mut MaybeUninit<T>,
}

impl<T: Send> Pusher<'_, T> {
    pub fn write(self, value: T) {
        unsafe { (*self.slot).write(value) };
        // self will be consumed here, then call the `drop` method
    }
}

impl<T: Send> Drop for Pusher<'_, T> {
    fn drop(&mut self) {
        self.producer.head += 1;
        self.producer
            .shared
            .head
            .0
            .store(self.producer.head, Ordering::Release);
    }
}

pub struct Popper<'a, T: Send> {
    consumer: &'a mut Consumer<T>,
    slot: *const MaybeUninit<T>,
}

impl<'a, T: Send> Deref for Popper<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { (*self.slot).assume_init_ref() }
    }
}

impl<T: Send> Drop for Popper<'_, T> {
    fn drop(&mut self) {
        unsafe { (*self.slot.cast_mut()).assume_init_drop() };

        self.consumer.tail += 1;
        self.consumer
            .shared
            .tail
            .0
            .store(self.consumer.tail, Ordering::Release);
    }
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

#[derive(Debug, PartialEq, Eq)]
pub struct FullError;

impl<T: Send> Producer<T> {
    pub fn push(&mut self) -> Result<Pusher<'_, T>, FullError> {
        if self.head - self.cache_tail == self.shared.capacity as u64 {
            // Update local cache of tail
            self.cache_tail = self.shared.tail.0.load(Ordering::Acquire);

            if self.head - self.cache_tail == self.shared.capacity as u64 {
                return Err(FullError);
            }
        }

        let index = self.head % self.shared.capacity as u64;
        let slot = unsafe { self.shared.buffer.get_unchecked(index as usize).get() };

        Ok(Pusher {
            producer: self,
            slot,
        })
    }
}

impl<T: Send> Consumer<T> {
    pub fn pop(&mut self) -> Option<Popper<'_, T>> {
        if self.tail == self.cache_head {
            // Update local cache of head
            self.cache_head = self.shared.head.0.load(Ordering::Acquire);

            if self.tail == self.cache_head {
                return None;
            }
        }

        let index = (self.tail % self.shared.capacity as u64) as usize;
        let slot = unsafe { self.shared.buffer.get_unchecked(index).get() };

        Some(Popper {
            consumer: self,
            slot,
        })
    }
}

impl<T: Send> Drop for Consumer<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
