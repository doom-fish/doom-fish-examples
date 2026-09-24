//! End-to-end: synthesise a 60-frame animated colour gradient on
//! `IOSurface`s, encode it with `videotoolbox`, mux through `avassetwriter`,
//! and write a verifiable .mp4 to /tmp.
//!
//! This wires apple-cf + videotoolbox + avassetwriter into the same
//! encoder→muxer pipeline you'd hook a screen capture into in production.
//!
//! Run with: `cargo run --bin capture_to_mp4`

use apple_cf::cm::CMTime;
use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;

const BGRA: u32 = u32::from_be_bytes(*b"BGRA");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width: i32 = 1280;
    let height: i32 = 720;
    let fps: i32 = 30;
    let total_frames: i32 = 60;
    let output = "/tmp/capture_to_mp4.mp4";
    let surface_width = usize::try_from(width)?;
    let surface_height = usize::try_from(height)?;

    println!("== Step 1: configure VideoToolbox encoder (H.264 @ 4 Mbps, {width}×{height} BGRA) ==");
    let encoder = CompressionSession::builder(width, height, Codec::H264)
        .with_real_time(true)
        .with_average_bit_rate(4_000_000)
        .with_expected_frame_rate(f64::from(fps))
        .with_max_keyframe_interval(fps)
        .build()?;

    println!("== Step 2: configure AVAssetWriter ==");
    let first_frame = encoder.encode(
        &gradient_surface(surface_width, surface_height, 0)?,
        CMTime::new(0, fps),
    )?;
    let first_sample = first_frame
        .cm_sample_buffer()
        .ok_or("encoder dropped the first frame")?;
    let writer = Writer::create(output, FileType::Mp4)?;
    let video_input = writer.add_video_input_from_sample(first_sample)?;
    writer.start_session(CMTime::new(0, fps))?;
    writer.append_sample(video_input, first_sample)?;

    println!("== Step 3: encode + mux {total_frames} frames ==");
    for i in 1..total_frames {
        let frame = encoder.encode(
            &gradient_surface(surface_width, surface_height, i)?,
            CMTime::new(i64::from(i), fps),
        )?;
        let sb = frame
            .cm_sample_buffer()
            .ok_or_else(|| format!("encoder dropped frame {i}"))?;
        writer.append_sample(video_input, sb)?;
    }

    writer.finish()?;
    let metadata = std::fs::metadata(output)?;
    println!(
        "\nOK Wrote {output} — {} bytes, {total_frames} frames @ {fps} fps ({}s)",
        metadata.len(),
        f64::from(total_frames) / f64::from(fps),
    );
    println!("    Verify with: ffprobe {output}");
    Ok(())
}

fn gradient_surface(width: usize, height: usize, frame_idx: i32) -> Result<IOSurface, String> {
    let surface = IOSurface::create(width, height, BGRA, 4).ok_or("IOSurface alloc")?;
    {
        let mut g = surface
            .lock(IOSurfaceLockOptions::NONE)
            .map_err(|c| format!("lock failed: {c}"))?;
        let bytes = unsafe { g.as_slice_mut() }.ok_or("non-contiguous surface")?;
        let phase = u8::try_from(frame_idx * 4 % 256).unwrap_or(0);
        for px in bytes.chunks_exact_mut(4) {
            px[0] = phase; // B
            px[1] = phase.wrapping_add(85); // G
            px[2] = phase.wrapping_add(170); // R
            px[3] = 0xFF;
        }
    }
    Ok(surface)
}
