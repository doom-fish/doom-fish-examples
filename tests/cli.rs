use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CAPTURE: &str = env!("CARGO_BIN_EXE_capture_to_mp4");
const OCR: &str = env!("CARGO_BIN_EXE_ocr_to_summary");

fn scratch_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

fn run(program: &str, args: &[&Path]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .expect("run example binary")
}

fn assert_is_mp4(path: &Path) {
    let bytes = fs::read(path).expect("read the written MP4");
    assert!(
        bytes.len() > 1024,
        "{} is only {} bytes",
        path.display(),
        bytes.len()
    );
    assert_eq!(&bytes[4..8], b"ftyp", "{} has no ftyp box", path.display());
}

#[test]
fn help_prints_usage_and_succeeds() {
    for (program, name) in [(CAPTURE, "capture_to_mp4"), (OCR, "ocr_to_summary")] {
        for flag in ["--help", "-h"] {
            let output = run(program, &[Path::new(flag)]);
            assert!(
                output.status.success(),
                "{name} {flag}: {:?}",
                output.status
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.starts_with(&format!("usage: {name} [")), "{stdout}");
        }
    }
}

#[test]
fn usage_errors_exit_with_status_2() {
    let cases: [&[&Path]; 2] = [&[Path::new("a"), Path::new("b")], &[Path::new("--bogus")]];
    for program in [CAPTURE, OCR] {
        for args in cases {
            let output = run(program, args);
            assert_eq!(output.status.code(), Some(2), "{program} {args:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("usage: "));
        }
    }
}

#[test]
fn capture_refuses_an_existing_output() {
    let dir = scratch_dir("existing-output");
    let output_path = dir.join("taken.mp4");
    fs::write(&output_path, b"keep").unwrap();

    let output = run(CAPTURE, &[&output_path]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read(&output_path).unwrap(), b"keep");
}

#[test]
fn capture_never_writes_through_a_symlink() {
    let dir = scratch_dir("symlinked-output");
    let victim = dir.join("victim");
    fs::write(&victim, b"keep").unwrap();
    let link = dir.join("link.mp4");
    symlink(&victim, &link).unwrap();
    let absent = dir.join("absent");
    let dangling = dir.join("dangling.mp4");
    symlink(&absent, &dangling).unwrap();

    assert_eq!(run(CAPTURE, &[&link]).status.code(), Some(1));
    assert_eq!(run(CAPTURE, &[&dangling]).status.code(), Some(1));

    assert_eq!(fs::read(&victim).unwrap(), b"keep");
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(fs::symlink_metadata(&absent).is_err());
}

#[test]
fn capture_writes_the_given_output() {
    let dir = scratch_dir("given-output");
    let output_path = dir.join("out.mp4");

    let output = run(CAPTURE, &[&output_path]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_is_mp4(&output_path);
}

#[test]
fn capture_defaults_to_a_new_private_directory_in_tmpdir() {
    let tmpdir = scratch_dir("default-output");

    let output = Command::new(CAPTURE)
        .env("TMPDIR", &tmpdir)
        .output()
        .expect("run capture_to_mp4");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let entries: Vec<PathBuf> = fs::read_dir(&tmpdir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "{entries:?}");
    let dir = &entries[0];
    let name = dir.file_name().unwrap().to_str().unwrap();
    let suffix = name.strip_prefix("capture_to_mp4-").unwrap();
    assert_eq!(suffix.len(), 16);
    assert!(suffix.bytes().all(|b| b.is_ascii_hexdigit()), "{name}");
    let metadata = fs::symlink_metadata(dir).unwrap();
    assert!(metadata.is_dir());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
    let mp4 = dir.join("capture_to_mp4.mp4");
    assert_is_mp4(&mp4);
    assert!(String::from_utf8_lossy(&output.stdout).contains(&mp4.display().to_string()));
}

#[test]
fn ocr_reports_an_unreadable_image() {
    let dir = scratch_dir("missing-image");

    let output = run(OCR, &[&dir.join("absent.png")]);

    assert_eq!(output.status.code(), Some(1));
}
