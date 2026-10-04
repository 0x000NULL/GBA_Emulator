//! CORE-ABI tests 1-6 (fruit-basket-ios/docs/CORE-ABI.md) against the jsmolka test ROMs and the
//! mGBA suite. ROMs come from `GBA_TESTS_DIR`, else `<repo>/tools/tests` (scripts/fetch-tests.ps1);
//! without either the ROM tests skip and say so, unless `GBA_TESTS_REQUIRED=1`.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use gba_ffi::*;

const W: usize = 240;
const H: usize = 160;

fn tests_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("GBA_TESTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tools/tests"));
    if dir.is_dir() {
        return Some(dir);
    }
    assert!(std::env::var("GBA_TESTS_REQUIRED").as_deref() != Ok("1"), "GBA_TESTS_DIR not set and tools/tests missing");
    eprintln!("skipped: GBA_TESTS_DIR not set (run scripts/fetch-tests.ps1)");
    None
}

fn rom(rel: &str) -> Option<Vec<u8>> {
    let path = tests_dir()?.join(rel);
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

fn create(rom: &[u8]) -> *mut FbCore {
    let mut st = FbStatus::ErrPanic;
    let core = unsafe { fb_gba_create(rom.as_ptr(), rom.len(), null(), 0, &mut st) };
    assert_eq!(st, FbStatus::Ok);
    assert!(!core.is_null());
    core
}

fn run(core: *mut FbCore, frames: usize) {
    for _ in 0..frames {
        assert_eq!(fb_gba_run_frame(core), FbStatus::Ok);
    }
}

fn picture(core: *const FbCore) -> &'static [u8] {
    let p = fb_gba_framebuffer(core);
    assert!(!p.is_null());
    unsafe { std::slice::from_raw_parts(p, W * H * 4) }
}

fn hash(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

fn save_state(core: *mut FbCore) -> Vec<u8> {
    let mut ptr = null_mut();
    let mut len = 0;
    assert_eq!(unsafe { fb_gba_state_save(core, &mut ptr, &mut len) }, FbStatus::Ok);
    let v = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
    unsafe { fb_gba_buffer_free(ptr, len) };
    v
}

// 1 ------------------------------------------------------------------------------------------

#[test]
fn t1_null_core_is_safe() {
    let n = null_mut::<FbCore>();
    let mut buf = [0u8; 16];
    let mut pcm = [0i16; 16];
    let mut ptr = null_mut();
    let mut len = 0usize;
    unsafe {
        fb_gba_destroy(n);
        assert_eq!(fb_gba_info(n), FbInfo::default());
        assert_eq!(fb_gba_reset(n), FbStatus::ErrArg);
        assert_eq!(fb_gba_run_frame(n), FbStatus::ErrArg);
        assert!(fb_gba_framebuffer(n).is_null());
        assert_eq!(fb_gba_audio_read(n, pcm.as_mut_ptr(), 8), 0);
        fb_gba_set_input(n, 0, 0x3FF);
        assert_eq!(fb_gba_sram_size(n), 0);
        assert_eq!(fb_gba_sram_read(n, buf.as_mut_ptr(), buf.len()), 0);
        assert_eq!(fb_gba_sram_write(n, buf.as_ptr(), buf.len()), FbStatus::ErrArg);
        assert!(!fb_gba_sram_dirty(n));
        assert_eq!(fb_gba_state_save(n, &mut ptr, &mut len), FbStatus::ErrArg);
        assert!(ptr.is_null());
        assert_eq!(fb_gba_state_load(n, buf.as_ptr(), buf.len()), FbStatus::ErrArg);
        fb_gba_buffer_free(null_mut(), 0);
        fb_gba_set_rtc(n, 0);
        fb_gba_set_sun_level(n, 5);
        // NULL arguments other than the core.
        let mut st = FbStatus::Ok;
        assert!(fb_gba_create(null(), 0, null(), 0, &mut st).is_null());
        assert_eq!(st, FbStatus::ErrArg);
        assert!(fb_gba_create(null(), 0, null(), 0, null_mut()).is_null());
    }
    assert_eq!(fb_gba_abi_version(), 1);
}

#[test]
fn t1_null_buffers_on_a_live_core() {
    let Some(r) = rom("gba-tests/save/sram.gba") else { return };
    let core = create(&r);
    unsafe {
        assert_eq!(fb_gba_audio_read(core, null_mut(), 8), 0);
        assert_eq!(fb_gba_sram_read(core, null_mut(), 8), 0);
        assert_eq!(fb_gba_sram_write(core, null(), 8), FbStatus::ErrArg);
        assert_eq!(fb_gba_state_save(core, null_mut(), null_mut()), FbStatus::ErrArg);
        assert_eq!(fb_gba_state_load(core, null(), 8), FbStatus::ErrArg);
        assert_eq!(fb_gba_state_load(core, null(), 0), FbStatus::ErrState);
        fb_gba_destroy(core);
    }
}

// 2 ------------------------------------------------------------------------------------------

#[test]
fn t2_picture_after_120_frames() {
    let Some(r) = rom("gba-tests/ppu/stripes.gba") else { return };
    let core = create(&r);
    let info = fb_gba_info(core);
    assert_eq!(
        info,
        FbInfo { width: 240, height: 160, fps_num: 16_777_216, fps_den: 280_896, sample_rate: 48_000, ports: 1 }
    );
    run(core, 120);
    let px = picture(core);
    assert!(px.as_chunks::<4>().0.iter().all(|p| p[3] == 255), "A = 255 throughout");
    assert!(px.as_chunks::<4>().0.iter().any(|p| p != &px[..4]), "the ROM drew something");
    unsafe { fb_gba_destroy(core) };
}

// 3 ------------------------------------------------------------------------------------------

#[test]
fn t3_audio_is_48k_per_emulated_second() {
    let Some(r) = rom("suite/suite.gba") else { return };
    let core = create(&r);
    let mut pcm = vec![0i16; 4096 * 2];
    let mut total = 0usize;
    for _ in 0..60 {
        run(core, 1);
        total += unsafe { fb_gba_audio_read(core, pcm.as_mut_ptr(), 4096) };
    }
    let secs = 60.0 * 280_896.0 / 16_777_216.0;
    let rate = total as f64 / secs;
    assert!((rate - 48_000.0).abs() <= 48_000.0 * 0.02, "{rate:.0} frames per emulated second");

    // Unread audio is capped at about 200 ms.
    run(core, 60);
    let kept = unsafe { fb_gba_audio_read(core, pcm.as_mut_ptr(), 4096) }
        + unsafe { fb_gba_audio_read(core, pcm.as_mut_ptr(), 4096) }
        + unsafe { fb_gba_audio_read(core, pcm.as_mut_ptr(), 4096) };
    assert_eq!(kept, 9600);
    unsafe { fb_gba_destroy(core) };
}

// 4 ------------------------------------------------------------------------------------------

#[test]
fn t4_state_round_trip() {
    let Some(r) = rom("suite/suite.gba") else { return };
    let core = create(&r);
    // Walk the suite's menu so the frames after the save actually change.
    run(core, 50);
    fb_gba_set_input(core, 0, 1 << 7); // Down
    run(core, 10);
    let state = save_state(core);
    eprintln!("state: {} bytes", state.len());
    fb_gba_set_input(core, 0, 0);
    run(core, 30);
    fb_gba_set_input(core, 0, 1); // A
    run(core, 30);
    let first = hash(picture(core));

    assert_eq!(unsafe { fb_gba_state_load(core, state.as_ptr(), state.len()) }, FbStatus::Ok);
    fb_gba_set_input(core, 0, 0);
    run(core, 30);
    fb_gba_set_input(core, 0, 1);
    run(core, 30);
    assert_eq!(hash(picture(core)), first);
    unsafe { fb_gba_destroy(core) };
}

// 5 ------------------------------------------------------------------------------------------

#[test]
fn t5_sram_round_trip() {
    let Some(r) = rom("gba-tests/save/sram.gba") else { return };

    // The test ROM writes SRAM: the flag rises, and a full read clears it.
    let core = create(&r);
    assert_eq!(fb_gba_sram_size(core), 0x8000);
    assert!(!fb_gba_sram_dirty(core));
    run(core, 60);
    assert!(fb_gba_sram_dirty(core));
    let mut short = [0u8; 16];
    assert_eq!(unsafe { fb_gba_sram_read(core, short.as_mut_ptr(), short.len()) }, 16);
    assert!(fb_gba_sram_dirty(core), "a partial read leaves the flag");
    let mut written = vec![0u8; 0x8000];
    assert_eq!(unsafe { fb_gba_sram_read(core, written.as_mut_ptr(), written.len()) }, 0x8000);
    assert!(!fb_gba_sram_dirty(core));
    assert!(written.iter().any(|&b| b != 0xFF), "the ROM wrote something");
    unsafe { fb_gba_destroy(core) };

    // A fresh core given that save reads it back unchanged.
    let core = create(&r);
    let pattern: Vec<u8> = (0..0x8000u32).map(|i| (i * 7 + 3) as u8).collect();
    assert_eq!(unsafe { fb_gba_sram_write(core, pattern.as_ptr(), 100) }, FbStatus::ErrSave);
    assert_eq!(unsafe { fb_gba_sram_write(core, pattern.as_ptr(), pattern.len()) }, FbStatus::Ok);
    assert!(!fb_gba_sram_dirty(core));
    let mut back = vec![0u8; 0x8000];
    assert_eq!(unsafe { fb_gba_sram_read(core, back.as_mut_ptr(), back.len()) }, 0x8000);
    assert_eq!(back, pattern);

    // Save memory survives reset.
    assert_eq!(fb_gba_reset(core), FbStatus::Ok);
    assert_eq!(unsafe { fb_gba_sram_read(core, back.as_mut_ptr(), back.len()) }, 0x8000);
    assert_eq!(back, pattern);
    unsafe { fb_gba_destroy(core) };

    // No battery: size 0, writes refused.
    let Some(r) = rom("gba-tests/save/none.gba") else { return };
    let core = create(&r);
    run(core, 10);
    assert_eq!(fb_gba_sram_size(core), 0);
    assert_eq!(unsafe { fb_gba_sram_read(core, back.as_mut_ptr(), back.len()) }, 0);
    assert_eq!(unsafe { fb_gba_sram_write(core, pattern.as_ptr(), pattern.len()) }, FbStatus::ErrSave);
    assert!(!fb_gba_sram_dirty(core));
    unsafe { fb_gba_destroy(core) };
}

// 6 ------------------------------------------------------------------------------------------

#[test]
fn t6_garbage_rom_and_bios() {
    let mut st = FbStatus::Ok;
    let garbage: Vec<u8> = (0..0x10000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    for len in [garbage.len(), 100] {
        let core = unsafe { fb_gba_create(garbage.as_ptr(), len, null(), 0, &mut st) };
        assert!(core.is_null());
        assert_eq!(st, FbStatus::ErrRom);
    }
    let huge = vec![0u8; 32 * 1024 * 1024 + 1];
    assert!(unsafe { fb_gba_create(huge.as_ptr(), huge.len(), null(), 0, &mut st) }.is_null());
    assert_eq!(st, FbStatus::ErrRom);

    let Some(r) = rom("gba-tests/ppu/stripes.gba") else { return };
    let bios = vec![0u8; 1000];
    assert!(unsafe { fb_gba_create(r.as_ptr(), r.len(), bios.as_ptr(), bios.len(), &mut st) }.is_null());
    assert_eq!(st, FbStatus::ErrBios);
    assert!(unsafe { fb_gba_create(r.as_ptr(), r.len(), null(), 16384, &mut st) }.is_null());
    assert_eq!(st, FbStatus::ErrArg);
}

#[test]
fn t6_foreign_state_is_rejected() {
    let Some(r) = rom("gba-tests/ppu/stripes.gba") else { return };
    let core = create(&r);
    run(core, 30);
    let before = hash(picture(core));
    let good = save_state(core);

    let random: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2_246_822_519) >> 11) as u8).collect();
    // Another core's state: its own magic, then a plausible body.
    let mut nes = b"NESS".to_vec();
    nes.extend_from_slice(&1u32.to_le_bytes());
    nes.extend_from_slice(&random);
    // Our magic with another version, and our own state cut short.
    let mut old = good.clone();
    old[4..8].copy_from_slice(&999u32.to_le_bytes());
    let cut = &good[..good.len() / 2];
    let mut ours_garbled = good[..8].to_vec();
    ours_garbled.extend_from_slice(&random);

    for bad in [&random[..], &nes[..], &old[..], cut, &ours_garbled[..], &[]] {
        assert_eq!(unsafe { fb_gba_state_load(core, bad.as_ptr(), bad.len()) }, FbStatus::ErrState);
    }
    // The machine is untouched and still runs.
    assert_eq!(hash(picture(core)), before);
    assert_eq!(unsafe { fb_gba_state_load(core, good.as_ptr(), good.len()) }, FbStatus::Ok);
    run(core, 5);
    unsafe { fb_gba_destroy(core) };
}

/// Timing for the Swift side: `GBA_ROM=<rom.gba> cargo test --release -p gba-ffi -- --ignored
/// --nocapture frame_time`.
#[test]
#[ignore]
fn frame_time() {
    let Some(path) = std::env::var_os("GBA_ROM") else { return };
    let r = std::fs::read(path).unwrap();
    let core = create(&r);
    run(core, 300);
    let t = std::time::Instant::now();
    let mut pcm = vec![0i16; 4096 * 2];
    for _ in 0..1200 {
        run(core, 1);
        unsafe { fb_gba_audio_read(core, pcm.as_mut_ptr(), 4096) };
    }
    let per = t.elapsed().as_secs_f64() * 1000.0 / 1200.0;
    let state = save_state(core);
    let t = std::time::Instant::now();
    for _ in 0..20 {
        save_state(core);
    }
    let save_ms = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
    let t = std::time::Instant::now();
    for _ in 0..20 {
        unsafe { fb_gba_state_load(core, state.as_ptr(), state.len()) };
    }
    let load_ms = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
    eprintln!("{per:.3} ms/frame, state {} bytes, save {save_ms:.2} ms, load {load_ms:.2} ms", state.len());
    unsafe { fb_gba_destroy(core) };
}
