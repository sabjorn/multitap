//! Generic ring buffer implementation
//!
//! This module provides the core `RingBuffer` type that works with
//! any storage backend implementing `BufferBackend`.

use core::marker::PhantomData;
use crate::{Num, BufferBackend, ReadHandle, WriteGuard};

/// Core ring buffer that works with any storage backend
///
/// The ring buffer maintains a write position and delegates actual
/// storage to the backend. This allows the same ring buffer logic
/// to work with arrays, external memory, mmap files, etc.
pub struct RingBuffer<T: Num, B: BufferBackend<T>> {
    pub(crate) backend: B,
    pub(crate) write_position: usize,
    _phantom: PhantomData<T>,
}

impl<T: Num, B: BufferBackend<T>> RingBuffer<T, B> {
    /// Create a new ring buffer with the given backend
    pub fn new(backend: B) -> Self {
        RingBuffer {
            backend,
            write_position: 0,
            _phantom: PhantomData,
        }
    }

    /// Get a mutable write guard for exclusive write access
    pub fn write(&mut self) -> WriteGuard<'_, T, B> {
        WriteGuard { buffer: self }
    }

    /// Create a new read handle at the specified position
    /// If position is None, uses the current write position
    pub fn get_read_handle(&self, position: Option<usize>) -> ReadHandle<T> {
        let pos = position.unwrap_or(self.write_position);
        ReadHandle {
            read_position: pos % self.backend.capacity(),
            _phantom: PhantomData,
        }
    }

    /// Get the current write position
    pub fn write_position(&self) -> usize {
        self.write_position
    }

    /// Get the buffer size
    pub fn len(&self) -> usize {
        self.backend.capacity()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.backend.clear();
    }
}

unsafe impl<T: Num, B: BufferBackend<T>> Send for RingBuffer<T, B> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayBackend, SliceBackend};

    #[test]
    fn test_new() {
        let buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());
        assert_eq!(buffer.write_position(), 0);
        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn test_basic_write_read_array() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        let mut read_handle = buffer.get_read_handle(Some(0));
        let mut reader = read_handle.read(&buffer);

        assert_eq!(reader.next(), 1.0);
        assert_eq!(reader.next(), 2.0);
        assert_eq!(reader.next(), 3.0);
    }

    #[test]
    fn test_basic_write_read_slice() {
        let mut external_memory = [0.0f32; 4];
        let mut buffer = RingBuffer::new(SliceBackend::new(&mut external_memory));

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0);
        }

        let mut read_handle = buffer.get_read_handle(Some(0));
        let mut reader = read_handle.read(&buffer);

        assert_eq!(reader.next(), 1.0);
        assert_eq!(reader.next(), 2.0);
        assert_eq!(reader.next(), 3.0);
    }

    #[test]
    fn test_multiple_read_handles() {
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

        {
            let mut reader1 = handle1.read(&buffer);
            assert_eq!(reader1.next(), 1.0);
        }

        {
            let mut reader2 = handle2.read(&buffer);
            assert_eq!(reader2.next(), 3.0);
        }
    }

    #[test]
    fn test_ring_wrapping() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 2>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
            writer.push(3.0); // wraps around
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut reader = handle.read(&buffer);

        assert_eq!(reader.next(), 3.0); // wrapped value
        assert_eq!(reader.next(), 2.0);
    }

    #[test]
    fn test_audio_delay_pattern() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 16>::new());

        let mut delay_tap_1 = buffer.get_read_handle(None);
        let mut delay_tap_2 = buffer.get_read_handle(None);

        for frame in 0..32 {
            let input_sample = (frame as f32) * 0.1;

            {
                let mut writer = buffer.write();
                writer.push(input_sample);
            }

            if frame >= 4 {
                let delayed_1 = {
                    let mut reader = delay_tap_1.read(&buffer);
                    reader.next()
                };

                if frame >= 8 {
                    let delayed_2 = {
                        let mut reader = delay_tap_2.read(&buffer);
                        reader.next()
                    };

                    assert!((delayed_2 - (delayed_1 - 0.4)).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn test_clear() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        {
            let mut writer = buffer.write();
            writer.push(1.0);
            writer.push(2.0);
        }

        buffer.clear();

        let mut handle = buffer.get_read_handle(Some(0));
        let mut reader = handle.read(&buffer);
        assert_eq!(reader.next(), 0.0);
        assert_eq!(reader.next(), 0.0);
    }

    #[test]
    fn test_write_position_tracking() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        assert_eq!(buffer.write_position(), 0);

        buffer.write().push(1.0);
        assert_eq!(buffer.write_position(), 1);

        buffer.write().push(2.0);
        assert_eq!(buffer.write_position(), 2);
    }

    #[test]
    fn test_get_read_handle_with_none_position() {
        let mut buffer = RingBuffer::new(ArrayBackend::<f32, 4>::new());

        buffer.write().push(1.0);
        buffer.write().push(2.0);

        // None should use current write position
        let handle = buffer.get_read_handle(None);
        assert_eq!(handle.position(), 2);
    }
}
