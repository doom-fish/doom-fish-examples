//! End-to-end: render a multi-line "page" of prose to a PNG, OCR it with
//! `vision`, then ask `foundation-models` to summarise the recognised text.
//!
//! This is the flagship doom-fish demo: it wires vision + foundation-models
//! into the kind of pipeline that's the whole point of the suite — a
//! "describe what's on the screen" loop that runs entirely on-device.
//!
//! Run with: `cargo run --bin ocr_to_summary`

use std::path::PathBuf;

use foundation_models::prelude::*;
use vision::prelude::*;
use vision::recognize_text::_test_helper_render_text_png;

const SAMPLE_TEXT: &str = "The Norse god Thor wielded Mjolnir";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let png_path: PathBuf = "/tmp/ocr_to_summary.png".into();

    println!("== Step 1: render sample text to PNG ==");
    // Use a wider canvas so OCR doesn't crop the leading 'T' / trailing 'r'.
    _test_helper_render_text_png(SAMPLE_TEXT, 1600, 240, &png_path)?;
    println!(
        "wrote {} ({} bytes)",
        png_path.display(),
        std::fs::metadata(&png_path)?.len()
    );

    println!("\n== Step 2: OCR with Apple Vision ==");
    let recognizer = TextRecognizer::new()
        .with_recognition_level(RecognitionLevel::Accurate)
        .with_language_correction(true);
    let observations = recognizer.recognize_in_path(&png_path)?;
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
        eprintln!("\nERROR: Vision found no text in the rendered image");
        return Err("OCR returned no text".into());
    }

    println!("\n== Step 3: ask FoundationModels to summarise ==");
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
    );
    let prompt = format!("Summarise:\n{recognised}");
    let summary = session.respond(&prompt)?;
    println!("\nLLM summary:\n  {summary}");
    Ok(())
}
