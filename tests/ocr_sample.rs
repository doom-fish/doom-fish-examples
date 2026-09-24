use std::path::Path;

use apple_vision::recognize_text::{RecognitionLevel, TextRecognizer};

#[test]
fn vision_reads_the_bundled_sample() {
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/ocr_sample.png");

    let observations = TextRecognizer::new()
        .with_recognition_level(RecognitionLevel::Accurate)
        .with_language_correction(false)
        .recognize_in_path(&sample)
        .expect("Vision reads the bundled sample");

    let lines: Vec<&str> = observations.iter().map(|o| o.text.as_str()).collect();
    assert_eq!(lines, ["The Norse god Thor wielded Mjolnir"]);
}
