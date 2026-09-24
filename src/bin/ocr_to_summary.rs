//! End-to-end: OCR an image of prose with `vision`, then ask
//! `foundation-models` to summarise the recognised text.
//!
//! This is the flagship doom-fish demo: it wires vision + foundation-models
//! into the kind of pipeline that's the whole point of the suite — a
//! "describe what's on the screen" loop that runs entirely on-device.
//!
//! Run with: `cargo run --bin ocr_to_summary [-- IMAGE]`

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apple_vision::prelude::*;
use foundation_models::prelude::*;

const SAMPLE_IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ocr_sample.png");

const USAGE: &str = "\
usage: ocr_to_summary [IMAGE]

Recognises the text in IMAGE with Apple Vision and asks the on-device
language model to summarise it. IMAGE defaults to the bundled
assets/ocr_sample.png. Nothing is written to disk.
";

enum Invocation {
    Help,
    Run(PathBuf),
}

fn main() -> ExitCode {
    let image = match parse_args(std::env::args_os().skip(1)) {
        Ok(Invocation::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Invocation::Run(image)) => image,
        Err(message) => {
            eprint!("ocr_to_summary: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&image) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ocr_to_summary: {error}");
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
        None => Ok(Invocation::Run(PathBuf::from(SAMPLE_IMAGE))),
        Some(arg) if arg == "-h" || arg == "--help" => Ok(Invocation::Help),
        Some(arg) if arg.as_encoded_bytes().starts_with(b"-") => {
            Err(format!("unknown option {}", arg.to_string_lossy()))
        }
        Some(arg) => Ok(Invocation::Run(PathBuf::from(arg))),
    }
}

fn run(image: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("== Step 1: OCR {} with Apple Vision ==", image.display());
    let recognizer = TextRecognizer::new()
        .with_recognition_level(RecognitionLevel::Accurate)
        .with_language_correction(true);
    let observations = recognizer.recognize_in_path(image)?;
    let recognised: String = observations
        .iter()
        .map(|o| o.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    println!("Vision returned {} observation(s):", observations.len());
    for obs in &observations {
        println!("  [{:.2}] {}", obs.confidence, obs.text);
    }
    if recognised.trim().is_empty() {
        return Err("Vision found no text in the image".into());
    }

    println!("\n== Step 2: ask FoundationModels to summarise ==");
    if !SystemLanguageModel::is_available() {
        let availability = SystemLanguageModel::availability();
        eprintln!(
            "SKIP: FoundationModels unavailable ({availability:?}).\n\
             Enable Apple Intelligence in System Settings to run the LLM step.\n\
             Would have asked the model to summarise this text:\n\
             {recognised}"
        );
        return Ok(());
    }

    let session = LanguageModelSession::with_instructions(
        "Summarise the supplied text in a single concise sentence under 20 words. \
         Don't repeat the text verbatim.",
    )?;
    let prompt = format!("Summarise:\n{recognised}");
    let summary = session.respond(&prompt)?;
    println!("\nLLM summary:\n  {summary}");
    Ok(())
}
