# doom-fish-examples

Cross-crate examples that showcase the [doom-fish](https://github.com/doom-fish) macOS Rust suite working as a system.

| Example | Stack | What it does |
|---|---|---|
| [`ocr_to_summary`](src/bin/ocr_to_summary.rs) | `vision` + `foundation-models` | Renders a line of prose to PNG, OCRs it via Apple Vision, asks the on-device LLM to summarize. |
| [`capture_to_mp4`](src/bin/capture_to_mp4.rs) | `apple-cf` + `videotoolbox` + `avassetwriter` | Synthesises a 60-frame BGRA gradient on an IOSurface, hardware-encodes to H.264, muxes into a `.mp4`. |

## The doom-fish stack

```
┌────────────────────────────────────────────────────────────────────┐
│  apple-cf-rs    cg • iosurface • dispatch • cm                     │
│       ▲                                                            │
│       ├─ videotoolbox-rs    encoder w/ &CMSampleBuffer             │
│       │       ▲                                                    │
│       │       └─ avassetwriter-rs   takes &CMSampleBuffer + audio  │
│       │                                                            │
│       ├─ foundation-models-rs       on-device LLM                  │
│       │                                                            │
│       └─ vision-rs                  OCR (VNRecognizeTextRequest)   │
│                                                                    │
│       └─► doom-fish-examples (this repo)                           │
└────────────────────────────────────────────────────────────────────┘
```

All five bridge crates compose with safe types — no `*mut c_void` leaks across crate boundaries, no `bindgen`, no `objc2`, no procedural macros.

## Layout

This crate assumes the doom-fish bridge crates live as siblings in `~/dev/`:

```
~/dev/
├── apple-cf-rs/
├── avassetwriter-rs/
├── doom-fish-examples/   ← this repo
├── foundation-models-rs/
├── videotoolbox-rs/
└── vision-rs/
```

The path-deps in `Cargo.toml` resolve relative to `..`. Once the bridge crates are published to crates.io this becomes a regular versioned dep.

## Running

```bash
git clone https://github.com/doom-fish/doom-fish-examples ~/dev/doom-fish-examples
# (and the five bridge crates as siblings)

cd ~/dev/doom-fish-examples
cargo run --bin capture_to_mp4
ffprobe /tmp/capture_to_mp4.mp4

cargo run --bin ocr_to_summary
```

`ocr_to_summary` requires Apple Intelligence enabled in System Settings to run the LLM step; without it the example gracefully skips and prints what it would have summarized.

## Why a separate examples repo

Cross-crate examples don't have a natural home in any single bridge crate without creating circular dev-dep chains. This repo is the integration point — it depends on every doom-fish crate at once and proves they all compose.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
