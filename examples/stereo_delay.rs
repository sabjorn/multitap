//! Stereo Delay Example - Multi-channel processing
//!
//! This example demonstrates processing stereo (2-channel) audio with
//! independent delay lines for each channel, creating a spacious stereo effect.
//!
//! Features:
//! - Independent delay times for left and right channels
//! - Cross-feedback between channels for wider stereo image
//! - Demonstrates multi-channel buffer management
//!
//! Usage:
//!   # Process stereo file
//!   sox input.wav -r 48000 -c 2 -t f32 - | \
//!     cargo run --example stereo_delay -- 200 300 0.3 0.6 | \
//!     play -t f32 -r 48000 -c 2 -
//!
//!   # Generate stereo test signal and process
//!   sox -n -r 48000 -c 2 -t f32 - synth 1 sine 440 sine 880 | \
//!     cargo run --example stereo_delay -- 150 250 0.4 0.5 | \
//!     play -t f32 -r 48000 -c 2 -
//!
//! Arguments:
//!   left_delay_ms   - Left channel delay in ms (default: 200)
//!   right_delay_ms  - Right channel delay in ms (default: 300)
//!   feedback        - Feedback amount 0.0-1.0 (default: 0.3)
//!   wet_mix         - Wet/dry mix 0.0-1.0 (default: 0.5)

use multitap::{RingBuffer, ArrayBackend};
use std::io::{self, Read, Write};

const SAMPLE_RATE: usize = 48000;

struct StereoDelayProcessor {
    left_buffer: RingBuffer<f32, ArrayBackend<f32, 16384>>,
    right_buffer: RingBuffer<f32, ArrayBackend<f32, 16384>>,
    left_tap: multitap::ReadHandle<f32>,
    right_tap: multitap::ReadHandle<f32>,
    feedback: f32,
    wet_mix: f32,
}

impl StereoDelayProcessor {
    fn new(feedback: f32, wet_mix: f32) -> Self {
        let left_buffer = RingBuffer::new(ArrayBackend::<f32, 16384>::new());
        let right_buffer = RingBuffer::new(ArrayBackend::<f32, 16384>::new());

        // Create delay taps at the current write position
        let left_tap = left_buffer.get_read_handle(None);
        let right_tap = right_buffer.get_read_handle(None);

        StereoDelayProcessor {
            left_buffer,
            right_buffer,
            left_tap,
            right_tap,
            feedback,
            wet_mix,
        }
    }

    fn process_frame(&mut self, left_in: f32, right_in: f32, sample_idx: usize,
                     left_delay: usize, right_delay: usize) -> (f32, f32) {
        // Read delayed samples
        let left_delayed = if sample_idx >= left_delay {
            self.left_tap.read(&self.left_buffer).next()
        } else {
            0.0
        };

        let right_delayed = if sample_idx >= right_delay {
            self.right_tap.read(&self.right_buffer).next()
        } else {
            0.0
        };

        // Cross-feedback: left delay gets some right signal and vice versa
        // This creates a wider stereo image
        let cross_amount = 0.3;
        let left_to_buffer = left_in +
            (left_delayed * self.feedback * (1.0 - cross_amount)) +
            (right_delayed * self.feedback * cross_amount);

        let right_to_buffer = right_in +
            (right_delayed * self.feedback * (1.0 - cross_amount)) +
            (left_delayed * self.feedback * cross_amount);

        // Write to buffers
        self.left_buffer.write().push(left_to_buffer);
        self.right_buffer.write().push(right_to_buffer);

        // Mix wet and dry signals
        let left_out = left_in * (1.0 - self.wet_mix) + left_delayed * self.wet_mix;
        let right_out = right_in * (1.0 - self.wet_mix) + right_delayed * self.wet_mix;

        (left_out, right_out)
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let left_delay_ms: f32 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200.0);

    let right_delay_ms: f32 = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(300.0);

    let feedback: f32 = args.get(3)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.3)
        .clamp(0.0, 0.9);

    let wet_mix: f32 = args.get(4)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);

    let left_delay_samples = ((SAMPLE_RATE as f32 * left_delay_ms) / 1000.0) as usize;
    let right_delay_samples = ((SAMPLE_RATE as f32 * right_delay_ms) / 1000.0) as usize;

    eprintln!("Stereo Delay Effect");
    eprintln!("  Sample Rate: {} Hz", SAMPLE_RATE);
    eprintln!("  Left Channel Delay: {:.1} ms ({} samples)", left_delay_ms, left_delay_samples);
    eprintln!("  Right Channel Delay: {:.1} ms ({} samples)", right_delay_ms, right_delay_samples);
    eprintln!("  Feedback: {:.1}%", feedback * 100.0);
    eprintln!("  Wet/Dry Mix: {:.1}%", wet_mix * 100.0);
    eprintln!("  Cross-feedback: 30% (for stereo width)");
    eprintln!();
    eprintln!("Reading stereo audio from stdin, writing to stdout...");

    let mut processor = StereoDelayProcessor::new(feedback, wet_mix);

    let mut input_bytes = [0u8; 8]; // 2 channels * 4 bytes per f32
    let mut sample_count = 0usize;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_handle = stdin.lock();
    let mut stdout_handle = stdout.lock();

    loop {
        // Read one stereo frame (2 f32 samples)
        match stdin_handle.read_exact(&mut input_bytes) {
            Ok(_) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }

        let left_in = f32::from_le_bytes([input_bytes[0], input_bytes[1], input_bytes[2], input_bytes[3]]);
        let right_in = f32::from_le_bytes([input_bytes[4], input_bytes[5], input_bytes[6], input_bytes[7]]);

        // Process the stereo frame
        let (left_out, right_out) = processor.process_frame(
            left_in,
            right_in,
            sample_count,
            left_delay_samples,
            right_delay_samples,
        );

        // Write output frame
        stdout_handle.write_all(&left_out.to_le_bytes())?;
        stdout_handle.write_all(&right_out.to_le_bytes())?;

        sample_count += 1;
    }

    eprintln!("Processed {} stereo frames ({:.2} seconds)",
              sample_count,
              sample_count as f32 / SAMPLE_RATE as f32);

    Ok(())
}
