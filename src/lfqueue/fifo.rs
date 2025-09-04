use std::mem::MaybeUninit;



pub struct Fifo1<T> {
    buffer: Vec<MaybeUninit<T>>,
    capacity: usize,
    head: u64,
    tail: u64,
}

impl<T> Fifo1<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);

        let mut buffer = Vec::with_capacity(capacity);

        unsafe {
            buffer.set_len(capacity);
        }

        Self {
            buffer,
            capacity,
            head: 0,
            tail: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.head.wrapping_sub(self.tail) == self.capacity as u64 {
            return Err(value);
        }

        let index = self.head % self.capacity as u64;

        unsafe {
            self.buffer.get_unchecked_mut(index as usize).write(value);
        }

        self.head += 1;

        Ok(())
    }


    pub fn pop(&mut self) -> Option<T> {
        if self.head == self.tail {
            return None;
        }

        let index = self.tail % self.capacity as u64;

        let value = unsafe {
            self.buffer.get_unchecked(index as usize).assume_init_read()
        };

        self.tail += 1;
        Some(value)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        (self.head - self.tail) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        self.len() == self.capacity
    }
}

