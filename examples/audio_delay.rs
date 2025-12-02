//! Audio Delay Example - Process raw audio from stdin to stdout
//!
//! This example demonstrates a practical audio delay effect.
//! It reads raw f32 audio samples from stdin and writes processed audio to stdout.
//!
//! Usage:
//!   # Generate test signal, process, and play
//!   sox -n -r 48000 -c 1 -t f32 - synth 0.5 sine 440 | \
//!     cargo run --example audio_delay -- 250 0.3 0.5 | \
//!     play -t f32 -r 48000 -c 1 -
//!
//!   # Or save to file
//!   sox -n -r 48000 -c 1 -t f32 - synth 1 sine 440 | \
//!     cargo run --example audio_delay -- 250 0.3 0.5 > output.raw
//!
//! Arguments:
//!   delay_ms  - Delay time in milliseconds (default: 250)
//!   feedback  - Feedback amount 0.0-1.0 (default: 0.3)
//!   wet_mix   - Wet/dry mix 0.0-1.0 (default: 0.5)

use multitap::{RingBuffer, ArrayBackend};
use std::io::{self, Read, Write};

const SAMPLE_RATE: usize = 48000; // 48kHz

fn main() -> io::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let delay_ms: f32 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(250.0);

    let feedback: f32 = args.get(2)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.3)
        .clamp(0.0, 1.0);

    let wet_mix: f32 = args.get(3)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);

    let delay_samples = ((SAMPLE_RATE as f32 * delay_ms) / 1000.0) as usize;

    // Print config to stderr (so it doesn't pollute stdout)
    eprintln!("Audio Delay Effect");
    eprintln!("  Sample Rate: {} Hz", SAMPLE_RATE);
    eprintln!("  Delay Time: {:.1} ms ({} samples)", delay_ms, delay_samples);
    eprintln!("  Feedback: {:.1}%", feedback * 100.0);
    eprintln!("  Wet/Dry Mix: {:.1}%", wet_mix * 100.0);
    eprintln!();
    eprintln!("Reading from stdin, writing to stdout...");

    // Create delay buffer (16384 samples = ~341ms at 48kHz)
    let mut delay_buffer = RingBuffer::new(ArrayBackend::<f32, 16384>::new());
    let mut delay_tap = delay_buffer.get_read_handle(None);

    // Read/write buffers
    let mut input_bytes = [0u8; 4]; // f32 is 4 bytes
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

        // Read delayed signal (if buffer has enough samples)
        let delayed = if sample_count >= delay_samples {
            delay_tap.read(&delay_buffer).next()
        } else {
            0.0 // Buffer still filling
        };

        // Apply feedback: mix input with delayed signal
        let to_buffer = input + (delayed * feedback);
        delay_buffer.write().push(to_buffer);

        // Mix wet (delayed) and dry (input) signals
        let output = input * (1.0 - wet_mix) + delayed * wet_mix;

        // Write output sample to stdout
        stdout_handle.write_all(&output.to_le_bytes())?;

        sample_count += 1;
    }

    eprintln!("Processed {} samples ({:.2} seconds)",
              sample_count,
              sample_count as f32 / SAMPLE_RATE as f32);

    Ok(())
}
