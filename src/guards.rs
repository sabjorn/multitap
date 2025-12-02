//! Write and read guards for safe buffer access
//!
//! Guards enforce exclusive write access and shared read access
//! through Rust's borrow checker.

use core::ops::{Index, IndexMut};
use crate::{Num, BufferBackend, RingBuffer};

/// Exclusive write access guard for RingBuffer
///
/// This guard provides mutable access to the ring buffer for writing.
/// Only one WriteGuard can exist at a time, enforced by the borrow checker.
pub struct WriteGuard<'a, T: Num, B: BufferBackend<T>> {
    pub(crate) buffer: &'a mut RingBuffer<T, B>,
}

impl<'a, T: Num, B: BufferBackend<T>> WriteGuard<'a, T, B> {
    /// Push a single element and advance the write position
    pub fn push(&mut self, element: T) {
        self.buffer.backend.set(self.buffer.write_position, element);
        self.increment();
    }

    /// Increment the write position
    pub fn increment(&mut self) {
        self.buffer.write_position =
            (self.buffer.write_position + 1) % self.buffer.backend.capacity();
    }

    /// Seek to a specific write position
    pub fn seek(&mut self, position: usize) {
        self.buffer.write_position = if position > self.buffer.backend.capacity() {
            0
        } else {
            position
        };
    }
}

impl<'a, T: Num, B: BufferBackend<T>> Index<usize> for WriteGuard<'a, T, B> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.buffer.backend.capacity();
        // We need to return a reference, but backend.get() returns T (a copy)
        // This is a fundamental limitation - we'll need to address this
        // For now, we'll use as_slice which all backends must provide
        &self.buffer.backend.as_slice()[current_position]
    }
}

impl<'a, T: Num, B: BufferBackend<T>> IndexMut<usize> for WriteGuard<'a, T, B> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % self.buffer.backend.capacity();
        &mut self.buffer.backend.as_mut_slice()[current_position]
    }
}

/// Temporary read guard - borrows the buffer for reading
pub struct ReadGuard<'a, T: Num> {
    pub(crate) buffer: &'a [T],
    pub(crate) size: usize,
    pub(crate) read_position: &'a mut usize,
}

impl<'a, T: Num> ReadGuard<'a, T> {
    /// Read the next sample and advance position
    pub fn next(&mut self) -> T {
        let sample = self.buffer[*self.read_position];
        *self.read_position = (*self.read_position + 1) % self.size;
        sample
    }

    /// Read at a specific offset without advancing position
    pub fn read_at(&self, offset: usize) -> T {
        let pos = offset % self.size;
        self.buffer[pos]
    }

    /// Get the current read position
    pub fn position(&self) -> usize {
        *self.read_position
    }
}

impl<'a, T: Num> Index<usize> for ReadGuard<'a, T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.size;
        &self.buffer[current_position]
    }
}

#[cfg(test)]
mod tests {
    use crate::{RingBuffer, ArrayBackend};

    #[test]
    fn test_write_guard_push() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        assert_eq!(buffer.write_position(), 3);
    }

    #[test]
    fn test_write_guard_increment() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.increment();
            writer.increment();
        }

        assert_eq!(buffer.write_position(), 2);
    }

    #[test]
    fn test_write_guard_seek() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.seek(2);
        }

        assert_eq!(buffer.write_position(), 2);

        // Test seeking beyond capacity resets to 0
        {
            let mut writer = buffer.write();
            writer.seek(10);
        }

        assert_eq!(buffer.write_position(), 0);
    }

    #[test]
    fn test_write_guard_index() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        {
            let writer = buffer.write();
            assert_eq!(writer[0], 1.0);
            assert_eq!(writer[1], 2.0);
            assert_eq!(writer[2], 3.0);
        }
    }

    #[test]
    fn test_write_guard_index_mut() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer[0] = 10.0;
            writer[1] = 20.0;
        }

        {
            let writer = buffer.write();
            assert_eq!(writer[0], 10.0);
            assert_eq!(writer[1], 20.0);
        }
    }

    #[test]
    fn test_write_guard_index_wraps() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer[0] = 1.0;
            writer[4] = 2.0; // wraps to index 0
        }

        {
            let writer = buffer.write();
            assert_eq!(writer[0], 2.0);
        }
    }

    #[test]
    fn test_read_guard_next() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut reader = handle.read(&buffer);

        assert_eq!(reader.next(), 1.0);
        assert_eq!(reader.next(), 2.0);
        assert_eq!(reader.next(), 3.0);
    }

    #[test]
    fn test_read_guard_read_at() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let reader = handle.read(&buffer);

        // read_at doesn't advance position
        assert_eq!(reader.read_at(0), 1.0);
        assert_eq!(reader.read_at(1), 2.0);
        assert_eq!(reader.read_at(2), 3.0);
        assert_eq!(reader.position(), 0); // Position unchanged
    }

    #[test]
    fn test_read_guard_position() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut reader = handle.read(&buffer);

        assert_eq!(reader.position(), 0);
        reader.next();
        assert_eq!(reader.position(), 1);
        reader.next();
        assert_eq!(reader.position(), 2);
    }

    #[test]
    fn test_read_guard_index() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let reader = handle.read(&buffer);

        assert_eq!(reader[0], 1.0);
        assert_eq!(reader[1], 2.0);
        assert_eq!(reader[2], 3.0);
    }

    #[test]
    fn test_read_guard_index_wraps() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 2>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let reader = handle.read(&buffer);

        assert_eq!(reader[0], 1.0);
        assert_eq!(reader[1], 2.0);
        assert_eq!(reader[2], 1.0); // wraps
        assert_eq!(reader[3], 2.0); // wraps
    }

    #[test]
    fn test_cannot_write_while_reading() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let reader = handle.read(&buffer);

        // This would fail to compile:
        // let mut writer = buffer.write(); // ERROR: cannot borrow mutably

        assert_eq!(reader[0], 1.0);

        drop(reader);

        // Now we can write again
        let mut writer = buffer.write();
        writer.push(2.0);
    }
}
