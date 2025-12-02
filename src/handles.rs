//! Read handles and iterators
//!
//! Read handles are persistent, owned types that track a read position
//! without borrowing the buffer. They can be stored and reused.

use core::marker::PhantomData;
use crate::{Num, BufferBackend, RingBuffer, ReadGuard};

/// A persistent handle for reading from a specific position in the buffer
///
/// This is owned and can be stored, but doesn't hold a reference to the buffer.
/// To actually read, you must create a `ReadGuard` by calling `read()`.
pub struct ReadHandle<T: Num> {
    pub(crate) read_position: usize,
    pub(crate) _phantom: PhantomData<T>,
}

unsafe impl<T: Num> Send for ReadHandle<T> {}

impl<T: Num> ReadHandle<T> {
    /// Seek to a specific read position
    pub fn seek(&mut self, position: usize) {
        self.read_position = position;
    }

    /// Move the read position by a delta (can be negative)
    pub fn delta(&mut self, position_delta: i64, buffer_size: usize) {
        self.read_position =
            ((self.read_position as i64 + position_delta) % buffer_size as i64) as usize;
    }

    /// Get the current read position
    pub fn position(&self) -> usize {
        self.read_position
    }

    /// Create a read guard for actual reading (borrows the buffer)
    pub fn read<'a, B: BufferBackend<T>>(
        &'a mut self,
        buffer: &'a RingBuffer<T, B>,
    ) -> ReadGuard<'a, T> {
        self.read_position = self.read_position % buffer.len();
        ReadGuard {
            buffer: buffer.backend.as_slice(),
            size: buffer.len(),
            read_position: &mut self.read_position,
        }
    }

    /// Iterator interface - creates a guard and returns an iterator
    pub fn iter<'a, B: BufferBackend<T>>(
        &'a mut self,
        buffer: &'a RingBuffer<T, B>,
        count: usize,
    ) -> ReadIterator<'a, T> {
        let guard = self.read(buffer);
        ReadIterator {
            guard,
            remaining: count,
        }
    }
}

/// Iterator that wraps a ReadGuard
pub struct ReadIterator<'a, T: Num> {
    pub(crate) guard: ReadGuard<'a, T>,
    pub(crate) remaining: usize,
}

impl<'a, T: Num> Iterator for ReadIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            Some(self.guard.next())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RingBuffer, ArrayBackend};

    #[test]
    fn test_read_handle_seek() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        assert_eq!(handle.position(), 0);

        handle.seek(2);
        assert_eq!(handle.position(), 2);

        let mut reader = handle.read(&buffer);
        assert_eq!(reader.next(), 3.0);
    }

    #[test]
    fn test_read_handle_delta_positive() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        handle.delta(2, buffer.len());

        assert_eq!(handle.position(), 2);

        let mut reader = handle.read(&buffer);
        assert_eq!(reader.next(), 3.0);
    }

    #[test]
    fn test_read_handle_delta_negative() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle = buffer.get_read_handle(Some(3));
        handle.delta(-2, buffer.len());

        assert_eq!(handle.position(), 1);

        let mut reader = handle.read(&buffer);
        assert_eq!(reader.next(), 2.0);
    }

    #[test]
    fn test_read_handle_delta_wraps() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        handle.delta(5, buffer.len()); // wraps around

        assert_eq!(handle.position(), 1);
    }

    #[test]
    fn test_read_handle_position() {
        let mut handle = ReadHandle::<f32> {
            read_position: 42,
            _phantom: PhantomData,
        };

        assert_eq!(handle.position(), 42);

        handle.seek(10);
        assert_eq!(handle.position(), 10);
    }

    #[test]
    fn test_read_iterator() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut iter = handle.iter(&buffer, 3);

        assert_eq!(iter.next(), Some(1.0));
        assert_eq!(iter.next(), Some(2.0));
        assert_eq!(iter.next(), Some(3.0));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_read_iterator_advances_handle_position() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));

        {
            let mut iter = handle.iter(&buffer, 2);
            // Consume the iterator
            while iter.next().is_some() {}
        }

        // Handle position should have advanced
        assert_eq!(handle.position(), 2);
    }

    #[test]
    fn test_read_iterator_zero_count() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut iter = handle.iter(&buffer, 0);

        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_read_iterator_wraps() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 2>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut iter = handle.iter(&buffer, 4); // More than buffer size

        assert_eq!(iter.next(), Some(1.0));
        assert_eq!(iter.next(), Some(2.0));
        assert_eq!(iter.next(), Some(1.0)); // wrapped
        assert_eq!(iter.next(), Some(2.0)); // wrapped
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_multiple_read_handles_independent() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
            writer.push(4.0);
        }

        let mut handle1 = buffer.get_read_handle(Some(0));
        let mut handle2 = buffer.get_read_handle(Some(2));

        // Advance handle1
        {
            let mut reader = handle1.read(&buffer);
            reader.next();
        }

        // handle2 should be unaffected
        assert_eq!(handle2.position(), 2);
        {
            let mut reader = handle2.read(&buffer);
            assert_eq!(reader.next(), 3.0);
        }
    }
}
