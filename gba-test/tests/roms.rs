//! `cargo test` driver: runs every `[[rom]]` in `tests/golden.toml` and asserts.
//!
//! Skips (and says so) when `GBA_TESTS_DIR` is unset, unless `GBA_TESTS_REQUIRED=1`.

use gba_test::checks::run_entry;
use gba_test::golden::GoldenFile;

#[test]
fn golden_roms() {
    let golden_path = gba_test::repo_root().join("tests").join("golden.toml");
    let file = GoldenFile::load(&golden_path).expect("tests/golden.toml parses");
    if gba_test::tests_dir().is_none() {
        let msg = "skipped: GBA_TESTS_DIR not set (run scripts/fetch-tests.ps1)";
        if std::env::var("GBA_TESTS_REQUIRED").map_or(false, |v| v == "1") {
            panic!("{msg}");
        }
        eprintln!("{msg}");
        return;
    }
    let mut failures = Vec::new();
    let mut ran = 0;
    for e in &file.roms {
        if let Some(reason) = &e.skip {
            eprintln!("SKIP {} ({reason})", e.path);
            continue;
        }
        ran += 1;
        match run_entry(&file, e) {
            Ok(out) => {
                let status = if out.passed() { "PASS" } else { "FAIL" };
                eprintln!("{status} {} frames={} hash={:016x} check={:?} golden_ok={:?}", e.path, out.frames, out.hash, out.check, out.golden_ok);
                if !out.passed() {
                    failures.push(format!("{}: {:?} golden_ok={:?} hash={:016x}\n  {}", e.path, out.check, out.golden_ok, out.hash, out.cpu_state));
                }
            }
            Err(err) => failures.push(format!("{}: error {err:#}", e.path)),
        }
    }
    assert!(failures.is_empty(), "{} of {ran} ROM entries failed:\n{}", failures.len(), failures.join("\n"));
}
