#![no_std]
use core::ops::{Index, IndexMut};
use core::marker::PhantomData;

pub trait Num: Copy + Send{
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

pub struct ReadHead<T: Num> {
    buffer : * const [T],
    size : usize,
    head_position : usize,
}

unsafe impl<T: Num> Send for ReadHead<T> {}

impl<T: Num> ReadHead<T> {
    pub fn seek(&mut self, position: usize) {
        self.head_position = position % self.size;
    }

    pub fn delta(&mut self, position_delta: i64) {
        self.head_position = ((self.head_position as i64 + position_delta) % self.size as i64) as usize;
    }
}

impl<T: Num> Iterator for ReadHead<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let sample: T;
        unsafe {
            sample = (*self.buffer)[self.head_position];
        }
        self.head_position = (self.head_position + 1) % (self.size);

        Some(sample)
    }
}

impl<T: Num> Index<usize> for ReadHead<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.size;
        unsafe {
            &(*self.buffer)[current_position]
        }
    }
}


pub struct WriteHead<T: Num, const N: usize> {
    buffer : [T; N],
    head_position : usize,
}

unsafe impl<T: Num, const N: usize> Send for WriteHead<T, N> {}

impl<T: Num, const N: usize> WriteHead<T, N> {
    pub fn new() -> WriteHead<T, N> {
        let buffer = [ T::default_value(); N];
        WriteHead {buffer, head_position: 0}
    }

    pub fn push(&mut self, element: T) {
        self.buffer[self.head_position] = element;
        self.increment();
    }
    
    pub fn increment(&mut self) {
        self.head_position = (self.head_position + 1) % self.buffer.len();
    }

    pub fn seek(&mut self, position: usize){
        self.head_position = if position > self.buffer.len() { 0 } else { position };
    }

    pub fn clear(&mut self) where T: Default {
        self.buffer.fill(T::default_value());
    }
    
    pub fn as_readhead(&self, delay_samples: usize) -> ReadHead<T> {
        ReadHead {buffer: self.buffer.as_slice(), size: self.buffer.len(), head_position: (self.buffer.len() - delay_samples) % self.buffer.len()}
    }
}


impl<T: Num, const N: usize> Iterator for WriteHead<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.buffer[self.head_position];
        self.increment();

        Some(sample)
    }
}

impl<T: Num, const N: usize> Index<usize> for WriteHead<T, N> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.buffer.len();
        &self.buffer[current_position]
    }
}

impl<T: Num, const N: usize> IndexMut<usize> for WriteHead<T, N> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % self.buffer.len();
        &mut self.buffer[current_position]
    }
}

pub struct WriteHeadOwned<T: Num> {
    buffer : *mut [T],
    capacity : usize,
    head_position : usize,
}

unsafe impl<T: Num> Send for WriteHeadOwned<T> {}

impl<T: Num> WriteHeadOwned<T> {
    pub fn new(buffer: &mut[T]) -> WriteHeadOwned<T> {
        let capacity = buffer.len();
        WriteHeadOwned {buffer, capacity, head_position: 0}
    }

    pub fn push(&mut self, element: T) {
        unsafe {
            (*self.buffer)[self.head_position] = element;
        }
        self.increment();
    }
    
    pub fn increment(&mut self) {
        self.head_position = (self.head_position + 1) % self.capacity;
    }

    pub fn seek(&mut self, position: usize){
        self.head_position = if position > self.capacity { 0 } else { position };
    }

    pub fn clear(&mut self) where T: Default {
        unsafe {
            for i in 0..self.capacity {
                (*self.buffer)[i] = T::default_value();
            }
        }
    }
    
    pub fn as_readhead(&self, delay_samples: usize) -> ReadHead<T> {
        ReadHead {buffer: self.buffer, size: self.capacity, head_position: (self.capacity - delay_samples) % self.capacity}
    }
}


impl<T: Num> Iterator for WriteHeadOwned<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let sample = (*self.buffer)[self.head_position];
            self.increment();
            Some(sample)
        }
    }
}

impl<T: Num> Index<usize> for WriteHeadOwned<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.capacity;
        unsafe {
            &(*self.buffer)[current_position]
        }
    }
}

impl<T: Num> IndexMut<usize> for WriteHeadOwned<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % self.capacity;
        unsafe {
            &mut (*self.buffer)[current_position]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn read_head_is_generic() {
        {
            let mut write_head = WriteHead::<i32, 1>::new();
            write_head.push(0);
        }
        {
            let mut write_head = WriteHead::<f32, 1>::new();
            write_head.push(0_f32);
        }
    }

    #[test]
    pub fn read_head_with_delay_output_equals_write_head() {
        let mut write_head = WriteHead::<i32, 3>::new();

        write_head.push(1);
        write_head.push(2);
        write_head.push(3);

        {
            let mut read_head = write_head.as_readhead(0);

            assert_eq!(read_head.next().unwrap(), 1);
            assert_eq!(read_head.next().unwrap(), 2);
            assert_eq!(read_head.next().unwrap(), 3);
        }

        {
            let mut read_head = write_head.as_readhead(1);

            assert_eq!(read_head.next().unwrap(), 3);
            assert_eq!(read_head.next().unwrap(), 1);
            assert_eq!(read_head.next().unwrap(), 2);
        }
        {
            let mut read_head = write_head.as_readhead(2);

            assert_eq!(read_head.next().unwrap(), 2);
            assert_eq!(read_head.next().unwrap(), 3);
            assert_eq!(read_head.next().unwrap(), 1);
        }
    }

    #[test]
    pub fn multiple_read_head_with_delay_output_equals_write_head() {
        let mut write_head = WriteHead::<f32, 5>::new();

        write_head.push(1.0);
        
        for n in 0..4 {
            let mut read_head_1 = write_head.as_readhead(n);
            let mut read_head_2 = write_head.as_readhead(n+1);
            for j in 0..4 {
                let val_1 = read_head_1.next().unwrap();
                let val_2 = read_head_2.next().unwrap();
                if j == n {
                    assert_eq!(val_1, 1.0)
                }
                if j == (n + 1) {
                    assert_eq!(val_2, 1.0)
                }
            }
        }
    }
    
    #[test]
    pub fn read_head_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();
        
        write_head.push(1.0);
        write_head.push(3.0);

        let mut read_head = write_head.as_readhead(0);
        
        assert_eq!(read_head.next().unwrap(), 1.0);
        assert_eq!(read_head.next().unwrap(), 3.0);
        assert_eq!(read_head.next().unwrap(), 1.0);
    }

    #[test]
    pub fn read_head_index_operator() {
        let mut write_head = WriteHead::<f32, 5>::new();

        for n in 0..4 {
            write_head.push(n as f32);
        }

        let read_head = write_head.as_readhead(0);
        for n in 0..4 {
            assert_eq!(read_head[n], n as f32);
        }
    }

    #[test]
    pub fn read_head_index_operator_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();

        write_head.push(0.0);
        write_head.push(1.0);

        let read_head = write_head.as_readhead(0);
        assert_eq!(read_head[0], 0.0);
        assert_eq!(read_head[1], 1.0);
        assert_eq!(read_head[2], 0.0);
        assert_eq!(read_head[3], 1.0);
        assert_eq!(read_head[4], 0.0);
    }

    #[test]
    pub fn read_head_seek() {
        let mut write_head = WriteHead::<f32, 2>::new();

        write_head.push(0.0);
        write_head.push(1.0);

        let mut read_head = write_head.as_readhead(0);
        assert_eq!(read_head.next().unwrap(), 0.0);

        read_head.seek(0);
        assert_eq!(read_head.next().unwrap(), 0.0);
    }

    #[test]
    pub fn read_head_seek_is_circuluar() {
        let mut write_head = WriteHead::<f32, 2>::new();

        write_head.push(0.0);
        write_head.push(1.0);

        let mut read_head = write_head.as_readhead(0);
        assert_eq!(read_head.next().unwrap(), 0.0);

        read_head.seek(2);

        assert_eq!(read_head.next().unwrap(), 0.0);
    }

    #[test]
    pub fn read_head_delta() {
        let mut write_head = WriteHead::<f32, 3>::new();

        write_head.push(1.0);
        write_head.push(2.0);
        write_head.push(3.0);

        let mut read_head = write_head.as_readhead(0);
        read_head.delta(2);

        assert_eq!(read_head.next().unwrap(), 3.0);
    }

    #[test]
    pub fn read_head_delta_no_change_if_zero() {
        let mut write_head = WriteHead::<f32, 3>::new();

        write_head.push(1.0);
        write_head.push(2.0);
        write_head.push(3.0);

        let mut read_head = write_head.as_readhead(0);
        read_head.delta(0);

        assert_eq!(read_head.next().unwrap(), 1.0);
    }

    #[test]
    pub fn read_head_delta_is_circuluar() {
        let mut write_head = WriteHead::<f32, 3>::new();

        write_head.push(1.0);
        write_head.push(2.0);
        write_head.push(3.0);

        let mut read_head = write_head.as_readhead(0);
        read_head.delta(3);

        assert_eq!(read_head.next().unwrap(), 1.0);
    }

    #[test]
    pub fn write_head_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();
        
        write_head.push(0.0);
        write_head.push(0.0);
        write_head.push(1.0); // wraps around

        let mut read_head = write_head.as_readhead(0);
        
        assert_eq!(read_head.next().unwrap(), 1.0);
        assert_eq!(read_head.next().unwrap(), 0.0);
    }
    
    #[test]
    pub fn write_head_index_operator() {
        let mut write_head = WriteHead::<f32, 5>::new();
        
        for n in 0..4 {
            write_head[n] = n as f32;
        }

        let mut read_head = write_head.as_readhead(0);
        for n in 0..4 {
            assert_eq!(read_head.next().unwrap(), n as f32);
        }
    }

    #[test]
    pub fn write_head_index_operator_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();
        
        write_head[0] = 0.0;
        write_head[1] = 1.0;
        write_head[2] = 2.0;
        write_head[3] = 3.0;

        let mut read_head = write_head.as_readhead(0);
        assert_eq!(read_head.next().unwrap(), 2.0);
        assert_eq!(read_head.next().unwrap(), 3.0);
    }

    #[test]
    pub fn moved_write_head_no_longer_valid() {
        let mut write_head = WriteHead::<f32, 2>::new();

        write_head[0] = 9.0;
        write_head[1] = 8.0;

        let mut read_head = write_head.as_readhead(0);

        let mut c = move || {
            write_head[0] = 0.0;
            write_head[1] = 1.0;
        };
        c();

        assert_eq!(read_head.next().unwrap(), 9.0);
        assert_eq!(read_head.next().unwrap(), 8.0);

    }

    #[test]
    pub fn write_head_owned_is_circular() {
        let mut array: [f32; 2] = [0.0; 2];
        let mut write_head = WriteHeadOwned::<f32>::new(&mut array);
        
        write_head.push(0.0);
        write_head.push(0.0);
        write_head.push(1.0); // wraps around

        let mut read_head = write_head.as_readhead(0);
        
        assert_eq!(read_head.next().unwrap(), 1.0);
        assert_eq!(read_head.next().unwrap(), 0.0);
    }
    
    #[test]
    pub fn write_head_owned_index_operator() {
        let mut array: [f32; 5] = [0.0; 5];
        let mut write_head = WriteHeadOwned::<f32>::new(&mut array);
        
        for n in 0..4 {
            write_head[n] = n as f32;
        }

        let mut read_head = write_head.as_readhead(0);
        for n in 0..4 {
            assert_eq!(read_head.next().unwrap(), n as f32);
        }
    }

    #[test]
    pub fn write_head_owned_index_operator_is_circular() {
        let mut array: [f32; 2] = [0.0; 2];
        let mut write_head = WriteHeadOwned::<f32>::new(&mut array);
        
        write_head[0] = 0.0;
        write_head[1] = 1.0;
        write_head[2] = 2.0;
        write_head[3] = 3.0;

        let mut read_head = write_head.as_readhead(0);
        assert_eq!(read_head.next().unwrap(), 2.0);
        assert_eq!(read_head.next().unwrap(), 3.0);
    }

   
    #[test]
    pub fn moved_write_head_owned_is_valid() {
        let mut array: [f32; 2] = [0.0; 2];
        let mut write_head = WriteHeadOwned::<f32>::new(&mut array);

        write_head[0] = 9.0;
        write_head[1] = 8.0;

        let mut read_head = write_head.as_readhead(0);

        let mut c = move || {
            write_head[0] = 0.0;
            write_head[1] = 1.0;
        };
        c();

        assert_eq!(read_head.next().unwrap(), 0.0);
        assert_eq!(read_head.next().unwrap(), 1.0);

    }
}

// ============================================================================
// New API - RingBuffer with separate WriteGuard and ReadHandle/ReadGuard
// ============================================================================
//
// # New API Design
//
// This API separates buffer ownership from read/write access, enforcing
// safety through Rust's borrow checker.
//
// ## Core Concepts
//
// - **`RingBuffer`**: Owns the buffer data (fixed-size array)
// - **`RingBufferOwned`**: References external memory (e.g., SDRAM)
// - **`WriteGuard`**: Exclusive mutable access for writing (borrows buffer)
// - **`ReadHandle`**: Persistent handle that tracks a read position (owned, no borrow)
// - **`ReadGuard`**: Temporary read access (borrows buffer and handle)
//
// ## Safety Guarantees
//
// The borrow checker enforces:
// - **One writer XOR multiple readers** (never both simultaneously)
// - Read handles can be stored and reused
// - No dangling pointers or data races
//
// ## Usage Examples
//
// ### Basic Delay Effect
//
// ```rust,ignore
// let mut buffer = RingBuffer::<f32, 1024>::new();
//
// // Create a delay tap (e.g., 500 samples delay)
// let mut delay_tap = buffer.get_read_handle(None);
//
// // In your audio processing loop:
// loop {
//     let input_sample = get_audio_input();
//
//     // Write input to buffer (exclusive access)
//     {
//         let mut writer = buffer.write();
//         writer.push(input_sample);
//     }
//
//     // Read delayed sample (shared access)
//     let delayed_sample = {
//         let mut reader = delay_tap.read(&buffer);
//         reader.next()
//     };
//
//     output_audio(input_sample + delayed_sample * 0.5);
// }
// ```
//
// ### Multiple Delay Taps
//
// ```rust,ignore
// let mut buffer = RingBuffer::<f32, 2048>::new();
//
// // Create multiple delay taps at different positions
// let mut tap_1 = buffer.get_read_handle(None);  // Current position
// let mut tap_2 = buffer.get_read_handle(None);  // Another tap
//
// // Read from multiple taps simultaneously (all share buffer)
// let sample_1 = tap_1.read(&buffer).next();
// let sample_2 = tap_2.read(&buffer).next();
// ```
//
// ### Iterator Interface
//
// ```rust,ignore
// let mut handle = buffer.get_read_handle(Some(0));
//
// // Process multiple samples at once
// for sample in handle.iter(&buffer, 128) {
//     process(sample);
// }
// ```
//
// ### External Memory (SDRAM)
//
// ```rust,ignore
// // Map SDRAM to a slice (unsafe, platform-specific)
// let sdram_slice = unsafe {
//     core::slice::from_raw_parts_mut(SDRAM_BASE as *mut f32, SDRAM_SIZE)
// };
//
// let mut buffer = RingBufferOwned::<f32>::new(sdram_slice);
// let mut handle = buffer.get_read_handle(None);
//
// // Use exactly like RingBuffer
// buffer.write().push(sample);
// let delayed = handle.read_owned(&buffer).next();
// ```

/// Core ring buffer that owns the data (fixed-size array)
pub struct RingBuffer<T: Num, const N: usize> {
    buffer: [T; N],
    write_position: usize,
}

impl<T: Num, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        RingBuffer {
            buffer: [T::default_value(); N],
            write_position: 0,
        }
    }

    /// Get a mutable write guard for exclusive write access
    pub fn write(&mut self) -> WriteGuard<'_, T, N> {
        WriteGuard { buffer: self }
    }

    /// Create a new read handle at the specified position
    /// If position is None, uses the current write position
    pub fn get_read_handle(&self, position: Option<usize>) -> ReadHandle<T> {
        let pos = position.unwrap_or(self.write_position);
        ReadHandle {
            read_position: pos % self.buffer.len(),
            _phantom: PhantomData,
        }
    }

    /// Get the current write position
    pub fn write_position(&self) -> usize {
        self.write_position
    }

    /// Get the buffer size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.fill(T::default_value());
    }
}

unsafe impl<T: Num, const N: usize> Send for RingBuffer<T, N> {}

/// Core ring buffer that references externally-owned memory (e.g., SDRAM)
pub struct RingBufferOwned<T: Num> {
    buffer: *mut [T],
    capacity: usize,
    write_position: usize,
}

impl<T: Num> RingBufferOwned<T> {
    pub fn new(buffer: &mut [T]) -> Self {
        let capacity = buffer.len();
        RingBufferOwned {
            buffer,
            capacity,
            write_position: 0,
        }
    }

    /// Get a mutable write guard for exclusive write access
    pub fn write(&mut self) -> WriteGuardOwned<'_, T> {
        WriteGuardOwned { buffer: self }
    }

    /// Create a new read handle at the specified position
    /// If position is None, uses the current write position
    pub fn get_read_handle(&self, position: Option<usize>) -> ReadHandle<T> {
        let pos = position.unwrap_or(self.write_position);
        ReadHandle {
            read_position: pos % self.capacity,
            _phantom: PhantomData,
        }
    }

    /// Get the current write position
    pub fn write_position(&self) -> usize {
        self.write_position
    }

    /// Get the buffer size
    pub fn len(&self) -> usize {
        self.capacity
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        unsafe {
            for i in 0..self.capacity {
                (*self.buffer)[i] = T::default_value();
            }
        }
    }
}

unsafe impl<T: Num> Send for RingBufferOwned<T> {}

/// Exclusive write access guard for RingBuffer
pub struct WriteGuard<'a, T: Num, const N: usize> {
    buffer: &'a mut RingBuffer<T, N>,
}

impl<'a, T: Num, const N: usize> WriteGuard<'a, T, N> {
    /// Push a single element and advance the write position
    pub fn push(&mut self, element: T) {
        self.buffer.buffer[self.buffer.write_position] = element;
        self.increment();
    }

    /// Increment the write position
    pub fn increment(&mut self) {
        self.buffer.write_position = (self.buffer.write_position + 1) % self.buffer.buffer.len();
    }

    /// Seek to a specific write position
    pub fn seek(&mut self, position: usize) {
        self.buffer.write_position = if position > self.buffer.buffer.len() {
            0
        } else {
            position
        };
    }
}

impl<'a, T: Num, const N: usize> Index<usize> for WriteGuard<'a, T, N> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.buffer.buffer.len();
        &self.buffer.buffer[current_position]
    }
}

impl<'a, T: Num, const N: usize> IndexMut<usize> for WriteGuard<'a, T, N> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % self.buffer.buffer.len();
        &mut self.buffer.buffer[current_position]
    }
}

/// Exclusive write access guard for RingBufferOwned
pub struct WriteGuardOwned<'a, T: Num> {
    buffer: &'a mut RingBufferOwned<T>,
}

impl<'a, T: Num> WriteGuardOwned<'a, T> {
    /// Push a single element and advance the write position
    pub fn push(&mut self, element: T) {
        unsafe {
            (*self.buffer.buffer)[self.buffer.write_position] = element;
        }
        self.increment();
    }

    /// Increment the write position
    pub fn increment(&mut self) {
        self.buffer.write_position = (self.buffer.write_position + 1) % self.buffer.capacity;
    }

    /// Seek to a specific write position
    pub fn seek(&mut self, position: usize) {
        self.buffer.write_position = if position > self.buffer.capacity {
            0
        } else {
            position
        };
    }
}

impl<'a, T: Num> Index<usize> for WriteGuardOwned<'a, T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.buffer.capacity;
        unsafe { &(*self.buffer.buffer)[current_position] }
    }
}

impl<'a, T: Num> IndexMut<usize> for WriteGuardOwned<'a, T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % self.buffer.capacity;
        unsafe { &mut (*self.buffer.buffer)[current_position] }
    }
}

/// A persistent handle for reading from a specific position in the buffer
/// This is owned and can be stored, but doesn't hold a reference to the buffer
pub struct ReadHandle<T: Num> {
    read_position: usize,
    _phantom: PhantomData<T>,
}

unsafe impl<T: Num> Send for ReadHandle<T> {}

impl<T: Num> ReadHandle<T> {
    /// Seek to a specific read position
    pub fn seek(&mut self, position: usize) {
        self.read_position = position;
    }

    /// Move the read position by a delta (can be negative)
    pub fn delta(&mut self, position_delta: i64, buffer_size: usize) {
        self.read_position = ((self.read_position as i64 + position_delta) % buffer_size as i64) as usize;
    }

    /// Get the current read position
    pub fn position(&self) -> usize {
        self.read_position
    }

    /// Create a read guard for actual reading (borrows the buffer)
    pub fn read<'a, const N: usize>(&'a mut self, buffer: &'a RingBuffer<T, N>) -> ReadGuard<'a, T> {
        self.read_position = self.read_position % buffer.len();
        ReadGuard {
            buffer: buffer.buffer.as_slice(),
            size: buffer.len(),
            read_position: &mut self.read_position,
        }
    }

    /// Create a read guard for RingBufferOwned
    pub fn read_owned<'a>(&'a mut self, buffer: &'a RingBufferOwned<T>) -> ReadGuard<'a, T> {
        self.read_position = self.read_position % buffer.len();
        ReadGuard {
            buffer: unsafe { &*buffer.buffer },
            size: buffer.len(),
            read_position: &mut self.read_position,
        }
    }

    /// Iterator interface - creates a guard and returns an iterator
    pub fn iter<'a, const N: usize>(&'a mut self, buffer: &'a RingBuffer<T, N>, count: usize) -> ReadIterator<'a, T> {
        let guard = self.read(buffer);
        ReadIterator {
            guard,
            remaining: count,
        }
    }

    /// Iterator interface for RingBufferOwned
    pub fn iter_owned<'a>(&'a mut self, buffer: &'a RingBufferOwned<T>, count: usize) -> ReadIterator<'a, T> {
        let guard = self.read_owned(buffer);
        ReadIterator {
            guard,
            remaining: count,
        }
    }
}

/// Temporary read guard - borrows the buffer for reading
pub struct ReadGuard<'a, T: Num> {
    buffer: &'a [T],
    size: usize,
    read_position: &'a mut usize,
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
        let pos = (offset) % self.size;
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

/// Iterator that wraps a ReadGuard
pub struct ReadIterator<'a, T: Num> {
    guard: ReadGuard<'a, T>,
    remaining: usize,
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
mod new_api_tests {
    use super::*;

    #[test]
    fn test_basic_write_read() {
        let mut buffer = RingBuffer::<f32, 4>::new();

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
        let mut buffer = RingBuffer::<f32, 4>::new();

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
    fn test_iterator_interface() {
        let mut buffer = RingBuffer::<f32, 4>::new();

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
    fn test_delay_effect() {
        let mut buffer = RingBuffer::<f32, 8>::new();

        // Write some samples
        {
            let mut writer = buffer.write();
            for i in 0..8 {
                writer.push(i as f32);
            }
        }

        // Create a delayed read handle (4 samples behind)
        let mut delay_handle = buffer.get_read_handle(Some(4));

        let mut reader = delay_handle.read(&buffer);
        assert_eq!(reader.next(), 4.0);
        assert_eq!(reader.next(), 5.0);
    }

    #[test]
    fn test_ring_wrapping() {
        let mut buffer = RingBuffer::<f32, 2>::new();

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
        // Simulates a typical audio delay effect pattern
        let mut buffer = RingBuffer::<f32, 16>::new();

        // Create two delay taps - one at 4 samples, one at 8 samples
        let mut delay_tap_1 = buffer.get_read_handle(None);
        let mut delay_tap_2 = buffer.get_read_handle(None);

        // Simulate processing audio frames
        for frame in 0..32 {
            let input_sample = (frame as f32) * 0.1;

            // Write to buffer
            {
                let mut writer = buffer.write();
                writer.push(input_sample);
            }

            // After buffer has some data, start reading delayed samples
            if frame >= 4 {
                // Read from delay taps
                let delayed_1 = {
                    let mut reader = delay_tap_1.read(&buffer);
                    reader.next()
                };

                // Second delay tap reads 4 samples later
                if frame >= 8 {
                    let delayed_2 = {
                        let mut reader = delay_tap_2.read(&buffer);
                        reader.next()
                    };

                    // Verify delay relationship (with floating point tolerance)
                    assert!((delayed_2 - (delayed_1 - 0.4)).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn test_cannot_write_while_reading() {
        // This test demonstrates that the borrow checker prevents
        // simultaneous write and read access
        let mut buffer = RingBuffer::<f32, 4>::new();

        {
            let mut writer = buffer.write();
            writer.push(1.0);
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let reader = handle.read(&buffer);

        // This would fail to compile:
        // let mut writer = buffer.write(); // ERROR: cannot borrow mutably while immutably borrowed

        // Read is safe
        assert_eq!(reader[0], 1.0);

        // Drop reader before writing again
        drop(reader);

        // Now we can write again
        let mut writer = buffer.write();
        writer.push(2.0);
    }

    #[test]
    fn test_owned_buffer_with_external_memory() {
        // Simulates using external memory (like SDRAM on embedded systems)
        let mut external_memory: [f32; 8] = [0.0; 8];

        let mut buffer = RingBufferOwned::<f32>::new(&mut external_memory);

        {
            let mut writer = buffer.write();
            for i in 0..4 {
                writer.push(i as f32);
            }
        }

        let mut handle = buffer.get_read_handle(Some(0));
        let mut reader = handle.read_owned(&buffer);

        assert_eq!(reader.next(), 0.0);
        assert_eq!(reader.next(), 1.0);
        assert_eq!(reader.next(), 2.0);
        assert_eq!(reader.next(), 3.0);

        // Verify the external memory was actually modified
        assert_eq!(external_memory[0], 0.0);
        assert_eq!(external_memory[1], 1.0);
    }
}
