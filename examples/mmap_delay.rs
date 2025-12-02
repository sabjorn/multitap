//! Memory-Mapped File Delay Example
//!
//! This example demonstrates:
//! 1. How to create a custom BufferBackend (MmapBackend)
//! 2. Using memory-mapped files for very large delay buffers
//! 3. The OS handling paging between RAM and disk for buffers larger than physical RAM
//!
//! Usage:
//!   sox /System/Library/Sounds/Glass.aiff -r 48000 -c 1 -t f32 - | \
//!     cargo run --example mmap_delay -- 5000 | \
//!     play -t f32 -r 48000 -c 1 -
//!
//! Arguments:
//!   delay_ms  - Delay time in milliseconds (default: 5000)

use multitap::{BufferBackend, Num, RingBuffer};
use memmap2::MmapMut;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

// Custom backend implementation using memory-mapped files
pub struct MmapBackend<T: Num> {
    mmap: MmapMut,
    capacity: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Num> MmapBackend<T> {
    pub fn create<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Self> {
        let size = capacity * std::mem::size_of::<T>();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.set_len(size as u64)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        Ok(MmapBackend {
            mmap,
            capacity,
            _phantom: std::marker::PhantomData,
        })
    }

    fn as_typed_slice(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                self.mmap.as_ptr() as *const T,
                self.capacity
            )
        }
    }

    fn as_typed_slice_mut(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.mmap.as_mut_ptr() as *mut T,
                self.capacity
            )
        }
    }
}

// Implement the BufferBackend trait for our custom backend
unsafe impl<T: Num> BufferBackend<T> for MmapBackend<T> {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn get(&self, index: usize) -> T {
        self.as_typed_slice()[index]
    }

    fn set(&mut self, index: usize, value: T) {
        self.as_typed_slice_mut()[index] = value;
    }

    fn as_slice(&self) -> &[T] {
        self.as_typed_slice()
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        self.as_typed_slice_mut()
    }

    fn clear(&mut self) {
        let slice = self.as_typed_slice_mut();
        for i in 0..slice.len() {
            slice[i] = T::default_value();
        }
    }
}

unsafe impl<T: Num> Send for MmapBackend<T> {}

const SAMPLE_RATE: usize = 48000;
const MMAP_FILE: &str = "/tmp/multitap_delay.mmap";

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let delay_ms: f32 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000.0);

    let delay_samples = ((SAMPLE_RATE as f32 * delay_ms) / 1000.0) as usize;

    // We need a buffer at least as large as the delay
    // Round up to next power of 2 for efficiency
    let buffer_size = delay_samples.next_power_of_two();

    eprintln!("Memory-Mapped Delay Effect");
    eprintln!("  Sample Rate: {} Hz", SAMPLE_RATE);
    eprintln!("  Delay Time: {:.1} ms ({} samples)", delay_ms, delay_samples);
    eprintln!("  Buffer Size: {} samples ({:.1} MB)",
              buffer_size,
              (buffer_size * 4) as f32 / 1_048_576.0);
    eprintln!("  Mmap File: {}", MMAP_FILE);
    eprintln!();

    // Create fresh mmap file
    eprintln!("Creating memory-mapped buffer...");
    let backend = MmapBackend::<f32>::create(MMAP_FILE, buffer_size)?;

    let mut buffer = RingBuffer::new(backend);
    let mut delay_tap = buffer.get_read_handle(None);

    eprintln!();
    eprintln!("Reading from stdin, writing to stdout...");
    eprintln!("Press Ctrl+C to stop");
    eprintln!();

    let feedback = 0.5;
    let wet_mix = 0.5;

    let mut input_bytes = [0u8; 4];
    let mut sample_count = 0usize;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_handle = stdin.lock();
    let mut stdout_handle = stdout.lock();

    loop {
        // Read one f32 sample from stdin
        match stdin_handle.read_exact(&mut input_bytes) {
            Ok(_) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }

        let input = f32::from_le_bytes(input_bytes);

        // Read delayed signal
        let delayed = if sample_count >= delay_samples {
            delay_tap.read(&buffer).next()
        } else {
            0.0
        };

        // Apply feedback
        let to_buffer = input + (delayed * feedback);
        buffer.write().push(to_buffer);

        // Mix wet and dry
        let output = input * (1.0 - wet_mix) + delayed * wet_mix;

        stdout_handle.write_all(&output.to_le_bytes())?;

        sample_count += 1;
    }

    eprintln!("Processed {} samples ({:.2} seconds)",
              sample_count,
              sample_count as f32 / SAMPLE_RATE as f32);

    // Clean up the mmap file
    fs::remove_file(MMAP_FILE)?;
    eprintln!("Cleaned up temporary mmap file");

    Ok(())
}
