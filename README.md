# doom-fish-examples

Cross-crate examples that showcase the [doom-fish](https://github.com/doom-fish) macOS Rust suite working as a system.

| Example | Stack | What it does |
|---|---|---|
| [`ocr_to_summary`](src/bin/ocr_to_summary.rs) | `apple-vision` + `foundation-models` | OCRs the bundled sample image (or an image you pass) with Apple Vision, then asks the on-device LLM to summarize the text. |
| [`capture_to_mp4`](src/bin/capture_to_mp4.rs) | `apple-cf` + `videotoolbox` + `avassetwriter` | Draws a 60-frame BGRA test pattern into IOSurfaces, encodes it to H.264 with VideoToolbox, and muxes it into an `.mp4`. |

## The doom-fish stack

```
apple-cf-rs            cg • iosurface • dispatch • cm • cv
├── videotoolbox-rs    encoder: &IOSurface in, EncodedFrame (CMSampleBuffer) out
├── avassetwriter-rs   muxer: takes &CMSampleBuffer (+ PCM audio)
└── vision-rs          OCR (VNRecognizeTextRequest)
foundation-models-rs   on-device LLM (does not use apple-cf)
```

All five bridge crates also use `doom-fish-utils` for shared FFI helpers. `capture_to_mp4` hands videotoolbox's `CMSampleBuffer`s straight to avassetwriter.

The crates hand each other safe wrapper types (`IOSurface`, `CMSampleBuffer`, `CMTime`) instead of raw pointers, and the examples contain one `unsafe` block: the pixel write into a newly created IOSurface in `capture_to_mp4`. Nothing runs `bindgen` at build time, but `apple_cf::raw` is `bindgen` output that apple-cf generated ahead of time and checked in. The dependency tree has no `objc2`; its only procedural macro is `serde_derive`, which avassetwriter and foundation-models use.

## Requirements

- Rust 1.82 or later, and Xcode.
- A macOS 26 build host, because foundation-models' Swift bridge targets macOS 26.
- `ocr_to_summary` runs on macOS 26 or later. `capture_to_mp4` uses crates that support macOS 13 or later.

## Layout

The five bridge crates and `doom-fish-utils` are path dependencies, so they must be checked out as sibling directories of this repository, in the same parent directory:

```
<parent>/
├── apple-cf-rs/
├── avassetwriter-rs/
├── doom-fish-examples/   ← this repo
├── doom-fish-utils/
├── foundation-models-rs/
├── videotoolbox-rs/
└── vision-rs/
```

Each path dependency also has a version requirement (for example `apple-cf >=0.11, <0.12`), so a sibling checkout at an incompatible version fails to resolve instead of building against the wrong API. The crate is not published to crates.io.

## Running

Clone this repository and the six siblings above into the same parent directory, then run from `doom-fish-examples/`:

```bash
cargo run --bin capture_to_mp4               # writes into a new private directory under $TMPDIR and prints the path
cargo run --bin capture_to_mp4 -- clip.mp4   # or writes clip.mp4, which must not exist yet
ffprobe clip.mp4

cargo run --bin ocr_to_summary               # OCRs assets/ocr_sample.png
cargo run --bin ocr_to_summary -- page.png   # or any image ImageIO can read
```

`capture_to_mp4` never overwrites anything: an output path that already exists, symlinks included, is an error. Its default directory is created with mode `0700` and an unpredictable name. `ocr_to_summary` writes nothing to disk.

`ocr_to_summary` requires Apple Intelligence enabled in System Settings to run the LLM step; without it the example gracefully skips and prints what it would have summarized.

Pass `--help` to either example for its usage. `cargo test` checks the argument handling, the output rules, a real encode, and that Vision reads the bundled sample; the tests keep their files under `target/`.

## Why a separate examples repo

Cross-crate examples don't have a natural home in any single bridge crate without creating circular dev-dep chains. This repo is the integration point: it depends on several doom-fish crates at once and shows that they compose.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.

That includes [`assets/ocr_sample.png`](assets/ocr_sample.png), which was made for this repository: the sample sentence, rendered once with Pillow in [Noto Sans](https://fonts.google.com/noto/specimen/Noto+Sans) Regular. Noto Sans is licensed under the SIL Open Font License 1.1, which does not extend to images made with the font.
