//! Slice-based storage backend for external memory
//!
//! This backend uses externally-owned memory via a raw pointer.
//! Useful for:
//! - Memory-mapped hardware (like SDRAM on embedded systems)
//! - Memory-mapped files for very large delays
//! - Pre-allocated memory pools

use crate::{Num, BufferBackend};

/// Backend using externally-owned memory (e.g., SDRAM, mmap)
pub struct SliceBackend<T: Num> {
    data: *mut [T],
    capacity: usize,
}

impl<T: Num> SliceBackend<T> {
    /// Create a new slice backend from a mutable slice
    ///
    /// # Safety
    ///
    /// The provided slice must remain valid for the lifetime of this backend.
    /// This is typically ensured by the caller maintaining ownership of the
    /// original memory.
    pub fn new(buffer: &mut [T]) -> Self {
        let capacity = buffer.len();
        SliceBackend {
            data: buffer as *mut [T],
            capacity,
        }
    }
}

unsafe impl<T: Num> BufferBackend<T> for SliceBackend<T> {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn get(&self, index: usize) -> T {
        unsafe { (*self.data)[index] }
    }

    fn set(&mut self, index: usize, value: T) {
        unsafe { (*self.data)[index] = value; }
    }

    fn as_slice(&self) -> &[T] {
        unsafe { &*self.data }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *self.data }
    }

    fn clear(&mut self) {
        unsafe {
            for i in 0..self.capacity {
                (*self.data)[i] = T::default_value();
            }
        }
    }
}

unsafe impl<T: Num> Send for SliceBackend<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let mut external_memory = [0.0f32; 4];
        let backend = SliceBackend::new(&mut external_memory);
        assert_eq!(backend.capacity(), 4);
        assert_eq!(backend.get(0), 0.0);
    }

    #[test]
    fn test_get_set() {
        let mut external_memory = [0.0f32; 4];
        let mut backend = SliceBackend::new(&mut external_memory);

        backend.set(0, 1.0);
        backend.set(1, 2.0);

        assert_eq!(backend.get(0), 1.0);
        assert_eq!(backend.get(1), 2.0);

        // Verify the external memory was actually modified
        assert_eq!(external_memory[0], 1.0);
        assert_eq!(external_memory[1], 2.0);
    }

    #[test]
    fn test_as_slice() {
        let mut external_memory = [0.0f32; 4];
        let mut backend = SliceBackend::new(&mut external_memory);

        backend.set(0, 1.0);
        backend.set(1, 2.0);

        let slice = backend.as_slice();
        assert_eq!(slice.len(), 4);
        assert_eq!(slice[0], 1.0);
        assert_eq!(slice[1], 2.0);
    }

    #[test]
    fn test_clear() {
        let mut external_memory = [1.0f32, 2.0, 3.0, 4.0];
        let mut backend = SliceBackend::new(&mut external_memory);

        backend.clear();

        assert_eq!(backend.get(0), 0.0);
        assert_eq!(backend.get(1), 0.0);
        assert_eq!(external_memory[0], 0.0);
        assert_eq!(external_memory[1], 0.0);
    }

    #[test]
    fn test_i32_type() {
        let mut external_memory = [0i32; 4];
        let mut backend = SliceBackend::new(&mut external_memory);

        backend.set(0, 42);
        assert_eq!(backend.get(0), 42);
        assert_eq!(external_memory[0], 42);
    }
}
