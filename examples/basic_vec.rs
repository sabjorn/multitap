//! Basic example: Using a Vec-backed ring buffer
//!
//! This example demonstrates how to use SliceBackendRuntime with a Vec
//! for heap-allocated buffers. This is useful when:
//! - Buffer size is determined at runtime
//! - Buffer is too large for stack allocation
//! - You're running on a system with std and heap allocation

use multitap::{RingBuffer, SliceBackendRuntime};

fn main() {
    // Create a large buffer on the heap (1 million samples)
    // This would be too large for stack allocation
    let buffer_size = 1_000_000;
    let mut vec_storage = vec![0.0f32; buffer_size];

    // Create a ring buffer using the Vec's slice
    // The Vec owns the memory, SliceBackendRuntime just stores the address
    let mut buffer = unsafe {
        RingBuffer::new(SliceBackendRuntime::from_slice(&mut vec_storage))
    };

    println!("Created ring buffer with {} samples", buffer.len());

    // Write some samples
    {
        let mut writer = buffer.write();
        for i in 0..10 {
            writer.push((i as f32) * 0.1);
        }
    }

    println!("Wrote 10 samples to the buffer");

    // Create a read handle at the beginning
    let mut read_handle = buffer.get_read_handle(Some(0));

    // Read back the samples
    println!("\nReading samples:");
    {
        let mut reader = read_handle.read(&buffer);
        for i in 0..10 {
            let sample = reader.next();
            println!("  Sample {}: {:.1}", i, sample);
        }
    }

    // Demonstrate delay effect: write input, read delayed output
    println!("\nDelay effect demonstration:");
    println!("(writing continuous samples, reading with 5 sample delay)\n");

    // Create a delay tap 5 samples behind the write head
    let mut delay_tap = buffer.get_read_handle(None);

    // Simulate processing 20 frames
    for frame in 0..20 {
        let input = (frame as f32) * 0.5;

        // Write input
        buffer.write().push(input);

        // Read delayed output (after 5 samples have been written)
        if frame >= 5 {
            let delayed = delay_tap.read(&buffer).next();
            println!("Frame {}: input={:.1}, delayed={:.1}", frame, input, delayed);
        } else {
            println!("Frame {}: input={:.1}, delayed=<filling buffer>", frame, input);
        }
    }

    // The Vec owns the memory and will be cleaned up when it goes out of scope
    println!("\nBuffer will be automatically cleaned up when Vec is dropped");
}
