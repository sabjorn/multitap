//! Low-Pass Filter Example - FIR convolution using ring buffer
//!
//! This example demonstrates a Finite Impulse Response (FIR) low-pass filter
//! implemented using a ring buffer for efficient convolution.
//!
//! A low-pass filter attenuates high frequencies while passing low frequencies.
//! This is useful for removing noise, smoothing signals, or anti-aliasing.
//!
//! Usage:
//!   # Process audio file with low-pass filter
//!   sox input.wav -r 48000 -c 1 -t f32 - | \
//!     cargo run --example lowpass_filter -- 1000 | \
//!     play -t f32 -r 48000 -c 1 -
//!
//!   # Try with a system sound
//!   sox /System/Library/Sounds/Glass.aiff -r 48000 -c 1 -t f32 - | \
//!     cargo run --example lowpass_filter -- 2000 | \
//!     play -t f32 -r 48000 -c 1 -
//!
//! Arguments:
//!   cutoff_hz - Cutoff frequency in Hz (default: 1000)

use multitap::{RingBuffer, ArrayBackend};
use std::io::{self, Read, Write};
use std::f32::consts::PI;

const SAMPLE_RATE: f32 = 48000.0;
const FILTER_TAPS: usize = 64; // Filter order

/// Generate a windowed sinc FIR low-pass filter
fn generate_lowpass_kernel(cutoff_hz: f32, num_taps: usize) -> Vec<f32> {
    let mut kernel = Vec::with_capacity(num_taps);
    let fc = cutoff_hz / SAMPLE_RATE; // Normalized cutoff frequency
    let center = (num_taps - 1) as f32 / 2.0;

    for i in 0..num_taps {
        let x = i as f32 - center;

        // Sinc function: sin(2πfcx) / (πx)
        let h = if x.abs() < 1e-6 {
            2.0 * fc // Limit as x approaches 0
        } else {
            (2.0 * PI * fc * x).sin() / (PI * x)
        };

        // Apply Hamming window to reduce ripple
        let window = 0.54 - 0.46 * (2.0 * PI * i as f32 / (num_taps - 1) as f32).cos();

        kernel.push(h * window);
    }

    // Normalize kernel so DC gain = 1
    let sum: f32 = kernel.iter().sum();
    for k in &mut kernel {
        *k /= sum;
    }

    kernel
}

fn main() -> io::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let cutoff_hz: f32 = args.get(1)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1000.0)
        .clamp(20.0, SAMPLE_RATE / 2.0);

    // Generate filter kernel
    let kernel = generate_lowpass_kernel(cutoff_hz, FILTER_TAPS);

    eprintln!("Low-Pass FIR Filter");
    eprintln!("  Sample Rate: {} Hz", SAMPLE_RATE);
    eprintln!("  Cutoff Frequency: {:.0} Hz", cutoff_hz);
    eprintln!("  Filter Taps: {}", FILTER_TAPS);
    eprintln!();
    eprintln!("Reading from stdin, writing to stdout...");

    // Create ring buffer to hold input sample history
    let mut sample_buffer = RingBuffer::new(ArrayBackend::<f32, FILTER_TAPS>::new());

    // Initialize buffer with zeros (for proper filter startup)
    for _ in 0..FILTER_TAPS {
        sample_buffer.write().push(0.0);
    }

    // Create read handle at the oldest sample position
    let mut read_handle = sample_buffer.get_read_handle(Some(0));

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

        // Write new sample to ring buffer
        sample_buffer.write().push(input);

        // Perform convolution: sum of (sample * kernel)
        let mut output = 0.0;
        {
            // Get iterator over the last FILTER_TAPS samples
            let mut iter = read_handle.iter(&sample_buffer, FILTER_TAPS);

            // Convolve with filter kernel
            for (i, sample) in (&mut iter).enumerate() {
                output += sample * kernel[i];
            }
        }

        // Write filtered output
        stdout_handle.write_all(&output.to_le_bytes())?;

        sample_count += 1;
    }

    eprintln!("Processed {} samples ({:.2} seconds)",
              sample_count,
              sample_count as f32 / SAMPLE_RATE);

    Ok(())
}
