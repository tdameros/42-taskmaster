use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer with a given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Add an element to the buffer, overwriting the oldest data if the buffer is full
    pub fn push(&mut self, value: T) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(value);
    }

    /// Delete all elements in the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Return a reference to all elements in order (FIFO)
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }
}
