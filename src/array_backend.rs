//! Array-based storage backend
//!
//! This backend uses a fixed-size stack-allocated array.
//! This is the most common choice for audio applications where
//! buffer size is known at compile time.

use crate::{Num, BufferBackend};

/// Backend using a fixed-size array (stack-allocated)
pub struct ArrayBackend<T: Num, const N: usize> {
    data: [T; N],
}

impl<T: Num, const N: usize> ArrayBackend<T, N> {
    /// Create a new array backend with default values
    pub fn new() -> Self {
        ArrayBackend {
            data: [T::default_value(); N],
        }
    }
}

unsafe impl<T: Num, const N: usize> BufferBackend<T> for ArrayBackend<T, N> {
    fn capacity(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> T {
        self.data[index]
    }

    fn set(&mut self, index: usize, value: T) {
        self.data[index] = value;
    }

    fn as_slice(&self) -> &[T] {
        &self.data
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    fn clear(&mut self) {
        self.data.fill(T::default_value());
    }
}

unsafe impl<T: Num, const N: usize> Send for ArrayBackend<T, N> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let backend = ArrayBackend::<f32, 4>::new();
        assert_eq!(backend.capacity(), 4);
        assert_eq!(backend.get(0), 0.0);
    }

    #[test]
    fn test_get_set() {
        let mut backend = ArrayBackend::<f32, 4>::new();
        backend.set(0, 1.0);
        backend.set(1, 2.0);
        assert_eq!(backend.get(0), 1.0);
        assert_eq!(backend.get(1), 2.0);
    }

    #[test]
    fn test_as_slice() {
        let mut backend = ArrayBackend::<f32, 4>::new();
        backend.set(0, 1.0);
        backend.set(1, 2.0);
        let slice = backend.as_slice();
        assert_eq!(slice.len(), 4);
        assert_eq!(slice[0], 1.0);
        assert_eq!(slice[1], 2.0);
    }

    #[test]
    fn test_clear() {
        let mut backend = ArrayBackend::<f32, 4>::new();
        backend.set(0, 1.0);
        backend.set(1, 2.0);
        backend.clear();
        assert_eq!(backend.get(0), 0.0);
        assert_eq!(backend.get(1), 0.0);
    }

    #[test]
    fn test_i32_type() {
        let mut backend = ArrayBackend::<i32, 4>::new();
        backend.set(0, 42);
        assert_eq!(backend.get(0), 42);
    }
}
