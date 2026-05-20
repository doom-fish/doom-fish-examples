//! End-to-end: synthesise a 60-frame animated colour gradient on an
//! `IOSurface`, encode it with `videotoolbox`, mux through `avassetwriter`,
//! and write a verifiable .mp4 to /tmp.
//!
//! This wires apple-cf + videotoolbox + avassetwriter into the same
//! encoder→muxer pipeline you'd hook a screen capture into in production.
//!
//! Run with: `cargo run --bin capture_to_mp4`

use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width: i32 = 1280;
    let height: i32 = 720;
    let fps: i32 = 30;
    let total_frames: i32 = 60;
    let pixel_format = u32::from_be_bytes(*b"BGRA");
    let output = "/tmp/capture_to_mp4.mp4";

    println!("== Step 1: allocate IOSurface ({width}×{height} BGRA) ==");
    let surface = IOSurface::create(
        usize::try_from(width)?,
        usize::try_from(height)?,
        pixel_format,
        4,
    )
    .ok_or("IOSurface alloc")?;

    println!("== Step 2: configure VideoToolbox encoder (H.264 @ 4 Mbps) ==");
    let encoder = CompressionSession::builder(width, height, Codec::H264)
        .with_real_time(true)
        .with_average_bit_rate(4_000_000)
        .with_expected_frame_rate(f64::from(fps))
        .with_max_keyframe_interval(fps)
        .build()?;

    println!("== Step 3: configure AVAssetWriter ==");
    fill_surface_gradient(&surface, 0)?;
    let first_frame = encoder.encode(&surface, (0, fps))?;
    let writer = Writer::create(output, FileType::Mp4)?;
    let video_input = writer.add_video_input_from_sample(
        first_frame
            .cm_sample_buffer()
            .expect("sample buffer for first frame"),
    )?;
    writer.start_session((0, fps))?;
    writer.append_sample(
        video_input,
        first_frame
            .cm_sample_buffer()
            .expect("sample buffer for first frame"),
    )?;

    println!("== Step 4: encode + mux {total_frames} frames ==");
    for i in 1..total_frames {
        fill_surface_gradient(&surface, i)?;
        let frame = encoder.encode(&surface, (i64::from(i), fps))?;
        let sb = frame.cm_sample_buffer().expect("sample buffer");
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

fn fill_surface_gradient(surface: &IOSurface, frame_idx: i32) -> Result<(), String> {
    let mut g = surface
        .lock(IOSurfaceLockOptions::NONE)
        .map_err(|c| format!("lock failed: {c}"))?;
    let bytes = g
        .as_slice_mut()
        .ok_or_else(|| "non-contiguous surface".to_string())?;
    let phase = u8::try_from(frame_idx * 4 % 256).unwrap_or(0);
    for px in bytes.chunks_exact_mut(4) {
        px[0] = phase; // B
        px[1] = phase.wrapping_add(85); // G
        px[2] = phase.wrapping_add(170); // R
        px[3] = 0xFF;
    }
    Ok(())
}
