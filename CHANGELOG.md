# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - Unreleased

### Security

- `capture_to_mp4` wrote to the fixed path `/tmp/capture_to_mp4.mp4` and `ocr_to_summary` to `/tmp/ocr_to_summary.png`, which any local user could pre-create, for example as a symlink to one of your files. `capture_to_mp4` now writes to a path given on the command line and refuses any path that already exists, symlinks included; by default it writes into a new directory under the temp directory, created with mode `0700` and an unpredictable name. `ocr_to_summary` writes nothing.

### Fixed

- `capture_to_mp4` did not compile: `IOSurfaceLockGuard::as_slice_mut` is `unsafe` since apple-cf 0.10, and its contract requires unique access to the surface. Each frame is now drawn into a new IOSurface before any other code has seen it, so the `unsafe` block meets that contract. Reusing one surface could not, because VideoToolbox may keep the pixel buffer that wraps it after `encode` returns.
- The crate did not resolve against the current sibling crates (its apple-vision requirement was `0.16.6`) and used their old APIs: tuple timestamps in videotoolbox and avassetwriter, and an infallible `LanguageModelSession::with_instructions`.
- A frame dropped by the encoder panicked in an `expect`; it is now reported as an error.
- `ocr_to_summary` depended on apple-vision's hidden `_test_helper_render_text_png`, which is not part of that crate's API. It now reads the bundled `assets/ocr_sample.png` or an image given on the command line.
- `build.rs` no longer adds the toolchain's `usr/lib/swift-5.5/macosx` directory to the rpath. It shadowed the SDK's `libswift_Concurrency.tbd` for the whole binary, and because it pointed into Xcode it never helped on other machines.
- `rust-version` claimed 1.70, below what the dependencies need.
- README: it told users to put the sibling crates in, and clone into, a fixed directory under their home folder; it now describes the sibling-directory layout neutrally and lists `doom-fish-utils`, which is needed too. It claimed "no bindgen" although `apple_cf::raw` is checked-in bindgen output, and "no procedural macros" although avassetwriter and foundation-models use `serde_derive`, and its diagram showed foundation-models built on apple-cf. It now states the build host and runtime requirements.

### Changed

- **Breaking:** `capture_to_mp4` takes an optional output path. Without one it writes into a new private directory under the temp directory and prints the path, instead of writing `/tmp/capture_to_mp4.mp4`. An existing output is an error instead of being replaced.
- **Breaking:** `ocr_to_summary` reads `assets/ocr_sample.png`, or the image given on the command line, instead of rendering text to `/tmp/ocr_to_summary.png`.
- `capture_to_mp4` creates the writer before it configures the encoder, so a bad output path fails before any encoding.
- Both examples print errors to stderr and exit with status 1, or with status 2 for usage errors.
- Requirements: apple-cf `>=0.11, <0.12`, apple-vision `>=0.17, <0.18`, avassetwriter `>=0.13, <0.14`, foundation-models `>=0.12, <0.13` and videotoolbox `>=0.21, <0.22`, still as path dependencies. `rust-version` is 1.82 (was 1.70).

### Added

- `--help` and `-h` for both examples.
- `assets/ocr_sample.png`: the sample sentence rendered in Noto Sans, which is licensed under the SIL Open Font License 1.1. Like the code, the image is licensed under MIT OR Apache-2.0.
- Tests for the argument handling, refusing existing and symlinked outputs, the private default directory, a real encode, and Vision reading the bundled sample.

## [0.1.0] - 2026-05-15

- Initial cross-crate examples: `ocr_to_summary` (apple-vision + foundation-models) and `capture_to_mp4` (apple-cf + videotoolbox + avassetwriter).
