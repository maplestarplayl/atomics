use std::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

struct Node<T> {
    data: Option<T>,
    next: AtomicPtr<Node<T>>,
}

pub struct LockFreeQueue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
}

impl<T> LockFreeQueue<T> {
    pub fn new() -> Self {
        let sentinel = Box::new(Node {
            data: None,
            next: AtomicPtr::new(std::ptr::null_mut()),
        });
        let sentinel_ptr = Box::into_raw(sentinel);

        let head = AtomicPtr::new(sentinel_ptr);
        let tail = AtomicPtr::new(sentinel_ptr);

        LockFreeQueue { head, tail }
    }

    pub fn push(&mut self, data: T) {
        let new_node = Box::new(Node {
            data: Some(data),
            next: AtomicPtr::new(std::ptr::null_mut()),
        });

        let new_node_ptr = Box::into_raw(new_node);

        loop {
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let tail_node = unsafe { &*tail_ptr };
            let next_ptr = tail_node.next.load(Ordering::Acquire);

            if !next_ptr.is_null() {
                let _ = self.tail.compare_exchange(
                    tail_ptr,
                    next_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
                continue;
            }

            let result = tail_node.next.compare_exchange(
                next_ptr,
                new_node_ptr,
                Ordering::Release,
                Ordering::Relaxed,
            );

            if result.is_ok() {
                let _ = self.tail.compare_exchange(
                    tail_ptr,
                    new_node_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                );

                break;
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        loop {
            let head_ptr = self.head.load(Ordering::Acquire);
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let head_node = unsafe { &*head_ptr };
            let next_ptr = head_node.next.load(Ordering::Acquire);

            if head_ptr == tail_ptr {
                if next_ptr.is_null() {
                    return None;
                }
                // Another thread is pushing
                let _ = self.tail.compare_exchange(
                    tail_ptr,
                    next_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
                continue;
            }

            if next_ptr.is_null() {
                continue;
            }

            let next_node = unsafe { &*next_ptr };
            let data = next_node.data.as_ref().unwrap();

            let result =
                self.head
                    .compare_exchange(head_ptr, next_ptr, Ordering::AcqRel, Ordering::Relaxed);

            if result.is_ok() {
                let old_head_box = unsafe { Box::from_raw(head_ptr) };

                // We can't move `data` out of `next_node` because it's behind a shared
                // reference. The safe way is to take the Option<T> out, leaving None.
                // However, since we are designing the queue, a simpler (but unsafe) way
                // is to read the data using `ptr::read`. This is safe *only* because we know
                // this node will become the new sentinel, and its data will never be read again.
                let data = unsafe { ptr::read(data) };

                // Let the Box go out of scope, freeing the memory of the old sentinel.
                drop(old_head_box);

                return Some(data);
            }
        }
    }
}

impl<T> Drop for LockFreeQueue<T> {
    fn drop(&mut self) {
        // Keep popping until the queue is empty.
        while let Some(_) = self.pop() {
            // The `pop` method handles deallocation of the old head node.
        }

        // Deallocate the final sentinel node.
        // `get_mut` gives us a mutable reference, which is safe because
        // we are in `drop` and have exclusive access.
        let sentinel_ptr = *self.head.get_mut();
        if !sentinel_ptr.is_null() {
            // Convert the last raw pointer back to a Box to be dropped.
            let _ = unsafe { Box::from_raw(sentinel_ptr) };
        }
    }
}
