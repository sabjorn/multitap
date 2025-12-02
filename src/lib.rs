#![no_std]

//! # Multitap - Zero-cost ring buffers for audio DSP
//!
//! A `no_std` ring buffer library designed for real-time audio processing,
//! featuring compile-time safety through Rust's borrow checker.
//!
//! ## Design Philosophy
//!
//! - **Storage abstraction**: Works with stack arrays, external memory (SDRAM), or mmap
//! - **Borrow checker safety**: Enforces one writer XOR multiple readers at compile time
//! - **Zero-cost**: No runtime overhead, everything monomorphizes
//! - **Real-time**: Wait-free, lock-free, no allocations
//!
//! ## Core Concepts
//!
//! - **`RingBuffer<T, B>`**: Main type, generic over storage backend `B`
//! - **`BufferBackend`**: Trait for storage (array, external memory, etc.)
//! - **`WriteGuard`**: Exclusive mutable access for writing
//! - **`ReadHandle`**: Persistent handle tracking a read position
//! - **`ReadGuard`**: Temporary immutable access for reading
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use multitap::{RingBuffer, ArrayBackend};
//!
//! // Create a ring buffer with stack-allocated array
//! let mut buffer = RingBuffer::new(ArrayBackend::<f32, 1024>::new());
//!
//! // Create a delay tap
//! let mut delay_tap = buffer.get_read_handle(None);
//!
//! // Audio processing loop
//! loop {
//!     let input = get_audio_sample();
//!
//!     // Write (exclusive access)
//!     buffer.write().push(input);
//!
//!     // Read delayed sample (shared access)
//!     let delayed = delay_tap.read(&buffer).next();
//!
//!     output_audio(input + delayed * 0.5);
//! }
//! ```
//!
//! ## Multiple Delay Taps
//!
//! ```rust,ignore
//! let mut buffer = RingBuffer::new(ArrayBackend::<f32, 2048>::new());
//!
//! let mut tap1 = buffer.get_read_handle(Some(100));  // 100 sample delay
//! let mut tap2 = buffer.get_read_handle(Some(500));  // 500 sample delay
//!
//! // Both can read simultaneously
//! let sample1 = tap1.read(&buffer).next();
//! let sample2 = tap2.read(&buffer).next();
//! ```
//!
//! ## External Memory (SDRAM)
//!
//! ```rust,ignore
//! use multitap::{RingBuffer, SliceBackend};
//!
//! // Map SDRAM to a slice (platform-specific, unsafe)
//! let sdram_slice = unsafe {
//!     core::slice::from_raw_parts_mut(SDRAM_BASE as *mut f32, SDRAM_SIZE)
//! };
//!
//! let mut buffer = RingBuffer::new(SliceBackend::new(sdram_slice));
//! // Use exactly like array-backed buffer
//! ```

// Re-export main types
pub use backend::BufferBackend;
pub use array_backend::ArrayBackend;
pub use slice_backend::SliceBackend;
pub use ring_buffer::RingBuffer;
pub use guards::{WriteGuard, ReadGuard};
pub use handles::{ReadHandle, ReadIterator};

// Module declarations
mod backend;
mod array_backend;
mod slice_backend;
mod ring_buffer;
mod guards;
mod handles;

/// Trait for numeric types that can be stored in ring buffers
pub trait Num: Copy + Send {
    fn default_value() -> Self;
}

impl Num for f32 {
    fn default_value() -> Self {
        0.0
    }
}

impl Num for i32 {
    fn default_value() -> Self {
        0
    }
}

// Convenience type aliases
pub type RingBufferArray<T, const N: usize> = RingBuffer<T, ArrayBackend<T, N>>;
pub type RingBufferOwned<T> = RingBuffer<T, SliceBackend<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_alias_array() {
        let mut buffer = RingBufferArray::<f32, 4>::new(ArrayBackend::new());
        buffer.write().push(42.0);

        let mut handle = buffer.get_read_handle(Some(0));
        assert_eq!(handle.read(&buffer).next(), 42.0);
    }

    #[test]
    fn test_type_alias_owned() {
        let mut external_memory = [0.0f32; 4];
        let mut buffer = RingBufferOwned::new(SliceBackend::new(&mut external_memory));

        buffer.write().push(42.0);

        let mut handle = buffer.get_read_handle(Some(0));
        assert_eq!(handle.read(&buffer).next(), 42.0);
    }
}
