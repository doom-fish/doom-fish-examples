//! End-to-end: synthesise a 60-frame animated colour gradient on
//! `IOSurface`s, encode it with `videotoolbox`, mux through `avassetwriter`,
//! and write a verifiable .mp4 to the path given on the command line, or to a
//! new private directory under the temp directory.
//!
//! This wires apple-cf + videotoolbox + avassetwriter into the same
//! encoder→muxer pipeline you'd hook a screen capture into in production.
//!
//! Run with: `cargo run --bin capture_to_mp4 [-- OUTPUT.mp4]`

use std::collections::hash_map::RandomState;
use std::ffi::OsString;
use std::fs::DirBuilder;
use std::hash::BuildHasher;
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::process::ExitCode;

use apple_cf::cm::CMTime;
use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;

const BGRA: u32 = u32::from_be_bytes(*b"BGRA");

const USAGE: &str = "\
usage: capture_to_mp4 [OUTPUT]

Encodes a 60-frame test pattern to H.264 and writes it as an MP4 file.
OUTPUT must not exist yet: nothing is ever overwritten. Without OUTPUT the
file goes into a new private directory under the temp directory, and its
path is printed.
";

enum Invocation {
    Help,
    Run(Option<PathBuf>),
}

fn main() -> ExitCode {
    let output = match parse_args(std::env::args_os().skip(1)) {
        Ok(Invocation::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Invocation::Run(output)) => output,
        Err(message) => {
            eprint!("capture_to_mp4: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("capture_to_mp4: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<Invocation, String> {
    let first = args.next();
    if args.next().is_some() {
        return Err("expected at most one argument".into());
    }
    match first {
        None => Ok(Invocation::Run(None)),
        Some(arg) if arg == "-h" || arg == "--help" => Ok(Invocation::Help),
        Some(arg) if arg.as_encoded_bytes().starts_with(b"-") => {
            Err(format!("unknown option {}", arg.to_string_lossy()))
        }
        Some(arg) => Ok(Invocation::Run(Some(PathBuf::from(arg)))),
    }
}

fn run(output: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let width: i32 = 1280;
    let height: i32 = 720;
    let fps: i32 = 30;
    let total_frames: i32 = 60;
    let surface_width = usize::try_from(width)?;
    let surface_height = usize::try_from(height)?;
    let output = match output {
        Some(output) => output,
        None => create_private_dir()?.join("capture_to_mp4.mp4"),
    };

    println!(
        "== Step 1: create AVAssetWriter for {} ==",
        output.display()
    );
    let writer = Writer::create(&output, FileType::Mp4)?;

    println!(
        "== Step 2: configure VideoToolbox encoder (H.264 @ 4 Mbps, {width}×{height} BGRA) =="
    );
    let encoder = CompressionSession::builder(width, height, Codec::H264)
        .with_real_time(true)
        .with_average_bit_rate(4_000_000)
        .with_expected_frame_rate(f64::from(fps))
        .with_max_keyframe_interval(fps)
        .build()?;

    println!("== Step 3: encode + mux {total_frames} frames ==");
    let first_frame = encoder.encode(
        &gradient_surface(surface_width, surface_height, 0)?,
        CMTime::new(0, fps),
    )?;
    let first_sample = first_frame
        .cm_sample_buffer()
        .ok_or("encoder dropped the first frame")?;
    let video_input = writer.add_video_input_from_sample(first_sample)?;
    writer.start_session(CMTime::new(0, fps))?;
    writer.append_sample(video_input, first_sample)?;
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
    let metadata = std::fs::metadata(&output)?;
    println!(
        "\nOK Wrote {} — {} bytes, {total_frames} frames @ {fps} fps ({}s)",
        output.display(),
        metadata.len(),
        f64::from(total_frames) / f64::from(fps),
    );
    println!("    Verify with: ffprobe {}", output.display());
    Ok(())
}

fn create_private_dir() -> io::Result<PathBuf> {
    let parent = std::env::temp_dir();
    let keys = RandomState::new();
    for attempt in 0..16_u32 {
        let suffix = keys.hash_one((std::process::id(), attempt));
        let dir = parent.join(format!("capture_to_mp4-{suffix:016x}"));
        match DirBuilder::new().mode(0o700).create(&dir) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            result => return result.map(|()| dir),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("found no unused directory name in {}", parent.display()),
    ))
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
