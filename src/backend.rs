//! Storage backend trait for ring buffers
//!
//! This module defines the `BufferBackend` trait that abstracts over
//! different storage mechanisms.

use crate::Num;

/// Trait for different storage backends (array, external memory, mmap, etc.)
///
/// This trait abstracts over the actual storage mechanism, allowing a single
/// RingBuffer implementation to work with different memory sources.
///
/// # Safety
///
/// Implementors must ensure:
/// - `capacity()` returns the actual capacity of the storage
/// - `get()` and `set()` are valid for indices 0..capacity()
/// - `as_slice()` returns a valid slice of length `capacity()`
pub unsafe trait BufferBackend<T: Num>: Send {
    /// Get the total capacity of the buffer
    fn capacity(&self) -> usize;

    /// Get a value at the given index (must be < capacity)
    fn get(&self, index: usize) -> T;

    /// Set a value at the given index (must be < capacity)
    fn set(&mut self, index: usize, value: T);

    /// Get a slice view of the entire buffer
    fn as_slice(&self) -> &[T];

    /// Get a mutable slice view of the entire buffer
    fn as_mut_slice(&mut self) -> &mut [T];

    /// Clear the buffer to default values
    fn clear(&mut self);
}
