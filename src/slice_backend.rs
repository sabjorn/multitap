//! Slice-based storage backend for external memory
//!
//! This module provides two backend types:
//! - `SliceBackend`: Compile-time const generic version (zero-sized, for hardware SDRAM)
//! - `SliceBackendRuntime`: Runtime version (stores address, for testing/flexibility)
//!
//! Both recreate pointers from the base address on every access, preventing
//! pointer invalidation issues when the backend struct is moved.
//!
//! Useful for:
//! - Memory-mapped hardware (like SDRAM on embedded systems)
//! - Memory-mapped files for very large delays
//! - Pre-allocated memory pools

use core::marker::PhantomData;
use crate::{Num, BufferBackend};

/// Backend using externally-owned memory at a compile-time fixed address
///
/// This is a zero-sized type that accesses memory at a compile-time known address.
/// The pointer is recreated from `BASE_ADDRESS` on every access, ensuring it never
/// goes stale even if the SliceBackend struct is moved.
///
/// # Type Parameters
///
/// - `T`: The element type
/// - `BASE_ADDRESS`: The memory address as a `usize` (e.g., `0xC0000000` for SDRAM)
/// - `CAPACITY`: The number of elements of type `T` in the memory region
///
/// # Safety
///
/// The caller must ensure that:
/// - The memory region from `BASE_ADDRESS` to `BASE_ADDRESS + CAPACITY * size_of::<T>()`
///   is valid and accessible
/// - No other code accesses this memory in a way that violates Rust's aliasing rules
/// - The memory remains valid for the lifetime of this backend
///
/// # Example
///
/// ```rust,ignore
/// // SDRAM at 0xC0000000, 64MB = 16M floats
/// let buffer = unsafe {
///     RingBuffer::new(SliceBackend::<f32, 0xC0000000, 16_777_216>::new())
/// };
/// ```
pub struct SliceBackend<T: Num, const BASE_ADDRESS: usize, const CAPACITY: usize> {
    _phantom: PhantomData<T>,
}

impl<T: Num, const BASE_ADDRESS: usize, const CAPACITY: usize> SliceBackend<T, BASE_ADDRESS, CAPACITY> {
    /// Create a new slice backend
    ///
    /// This is a const function that creates a zero-sized backend.
    /// The actual memory access happens through the const generic parameters.
    ///
    /// # Safety
    ///
    /// See the safety requirements on the type itself.
    pub const unsafe fn new() -> Self {
        SliceBackend {
            _phantom: PhantomData,
        }
    }
}

unsafe impl<T: Num, const BASE_ADDRESS: usize, const CAPACITY: usize> BufferBackend<T>
    for SliceBackend<T, BASE_ADDRESS, CAPACITY>
{
    fn capacity(&self) -> usize {
        CAPACITY
    }

    fn get(&self, index: usize) -> T {
        unsafe {
            let ptr = BASE_ADDRESS as *const T;
            *ptr.add(index)
        }
    }

    fn set(&mut self, index: usize, value: T) {
        unsafe {
            let ptr = BASE_ADDRESS as *mut T;
            *ptr.add(index) = value;
        }
    }

    fn as_slice(&self) -> &[T] {
        unsafe {
            core::slice::from_raw_parts(BASE_ADDRESS as *const T, CAPACITY)
        }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            core::slice::from_raw_parts_mut(BASE_ADDRESS as *mut T, CAPACITY)
        }
    }

    fn clear(&mut self) {
        unsafe {
            let ptr = BASE_ADDRESS as *mut T;
            for i in 0..CAPACITY {
                *ptr.add(i) = T::default_value();
            }
        }
    }
}

unsafe impl<T: Num, const BASE_ADDRESS: usize, const CAPACITY: usize> Send
    for SliceBackend<T, BASE_ADDRESS, CAPACITY> {}

/// Runtime version of SliceBackend for dynamic addresses
///
/// This version stores the base address and capacity at runtime, making it suitable
/// for situations where the address isn't known at compile time (like tests or
/// dynamic memory allocation).
///
/// The pointer is still recreated from the base address on every access, preventing
/// invalidation when the struct is moved.
///
/// # Safety
///
/// The caller must ensure that:
/// - The memory region from `base_address` to `base_address + capacity * size_of::<T>()`
///   is valid and accessible
/// - No other code accesses this memory in a way that violates Rust's aliasing rules
/// - The memory remains valid for the lifetime of this backend
pub struct SliceBackendRuntime<T: Num> {
    base_address: usize,
    capacity: usize,
    _phantom: PhantomData<T>,
}

impl<T: Num> SliceBackendRuntime<T> {
    /// Create a new runtime slice backend from a base address and capacity
    ///
    /// # Safety
    ///
    /// See the safety requirements on the type itself.
    pub unsafe fn new(base_address: usize, capacity: usize) -> Self {
        SliceBackendRuntime {
            base_address,
            capacity,
            _phantom: PhantomData,
        }
    }

    /// Create from a mutable slice
    ///
    /// This is a convenience wrapper that extracts the address and length from a slice.
    ///
    /// # Safety
    ///
    /// The memory must remain valid and at a stable address for the lifetime of this backend.
    pub unsafe fn from_slice(slice: &mut [T]) -> Self {
        Self::new(slice.as_mut_ptr() as usize, slice.len())
    }
}

unsafe impl<T: Num> BufferBackend<T> for SliceBackendRuntime<T> {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn get(&self, index: usize) -> T {
        unsafe {
            let ptr = self.base_address as *const T;
            *ptr.add(index)
        }
    }

    fn set(&mut self, index: usize, value: T) {
        unsafe {
            let ptr = self.base_address as *mut T;
            *ptr.add(index) = value;
        }
    }

    fn as_slice(&self) -> &[T] {
        unsafe {
            core::slice::from_raw_parts(self.base_address as *const T, self.capacity)
        }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            core::slice::from_raw_parts_mut(self.base_address as *mut T, self.capacity)
        }
    }

    fn clear(&mut self) {
        unsafe {
            let ptr = self.base_address as *mut T;
            for i in 0..self.capacity {
                *ptr.add(i) = T::default_value();
            }
        }
    }
}

unsafe impl<T: Num> Send for SliceBackendRuntime<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for the const generic SliceBackend
    #[test]
    fn test_slice_backend_zero_sized() {
        unsafe {
            let backend = SliceBackend::<f32, 0xC0000000, 1024>::new();
            // Verify it's truly zero-sized
            assert_eq!(core::mem::size_of_val(&backend), 0);
            assert_eq!(backend.capacity(), 1024);
        }
    }

    #[test]
    fn test_slice_backend_const_new() {
        // Test that new() is truly const
        const unsafe fn make_backend() -> SliceBackend<f32, 0xD0000000, 512> {
            SliceBackend::new()
        }

        unsafe {
            let _backend = make_backend();
        }
    }

    // Tests for SliceBackendRuntime
    #[test]
    fn test_runtime_new() {
        let mut external_memory = [0.0f32; 4];
        let backend = unsafe { SliceBackendRuntime::from_slice(&mut external_memory) };
        assert_eq!(backend.capacity(), 4);
        assert_eq!(backend.get(0), 0.0);
    }

    #[test]
    fn test_runtime_get_set() {
        let mut external_memory = [0.0f32; 4];
        let mut backend = unsafe { SliceBackendRuntime::from_slice(&mut external_memory) };

        backend.set(0, 1.0);
        backend.set(1, 2.0);

        assert_eq!(backend.get(0), 1.0);
        assert_eq!(backend.get(1), 2.0);

        // Verify the external memory was actually modified
        assert_eq!(external_memory[0], 1.0);
        assert_eq!(external_memory[1], 2.0);
    }

    #[test]
    fn test_runtime_as_slice() {
        let mut external_memory = [0.0f32; 4];
        let mut backend = unsafe { SliceBackendRuntime::from_slice(&mut external_memory) };

        backend.set(0, 1.0);
        backend.set(1, 2.0);

        let slice = backend.as_slice();
        assert_eq!(slice.len(), 4);
        assert_eq!(slice[0], 1.0);
        assert_eq!(slice[1], 2.0);
    }

    #[test]
    fn test_runtime_clear() {
        let mut external_memory = [1.0f32, 2.0, 3.0, 4.0];
        let mut backend = unsafe { SliceBackendRuntime::from_slice(&mut external_memory) };

        backend.clear();

        assert_eq!(backend.get(0), 0.0);
        assert_eq!(backend.get(1), 0.0);
        assert_eq!(external_memory[0], 0.0);
        assert_eq!(external_memory[1], 0.0);
    }

    #[test]
    fn test_runtime_i32_type() {
        let mut external_memory = [0i32; 4];
        let mut backend = unsafe { SliceBackendRuntime::from_slice(&mut external_memory) };

        backend.set(0, 42);
        assert_eq!(backend.get(0), 42);
        assert_eq!(external_memory[0], 42);
    }
}
