//! gba-ffi: Strawberry's core behind the Fruit Basket iOS core C ABI, v1, prefix `fb_gba_`.
//!
//! The contract is `fruit-basket-ios/docs/CORE-ABI.md`. This crate is an rlib: the app's single
//! staticlib (`fbcore`) depends on it and re-exports the symbols.
//!
//! Every entry point accepts a NULL core, runs its body under `catch_unwind`, and poisons the core
//! after a panic (every later call returns `FB_ERR_PANIC` or zero until `destroy`). The picture is
//! converted BGR555 -> RGBA8888 once per `run_frame`; audio is resampled to 48 kHz into a ring
//! capped at 200 ms.
//!
//! Beyond what gba-core checks (size, BIOS length), `create` rejects a ROM whose header fails both
//! of the BIOS's checks: the complement checksum at 0xBD and the fixed 0x96 at 0xB2. Either one
//! passing is enough, so homebrew never run through gbafix (bad checksum) still boots; garbage
//! fails both about 255/256 x 255/256 of the time.

mod audio;

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;

use audio::{Audio, OUT_RATE};
use gba_core::cart::{ROM_MAX, SRAM_SIZE};
use gba_core::timing::{CLOCK_HZ, CYCLES_PER_FRAME};
use gba_core::{BiosMode, Gba, Keys, Options, SCREEN_H, SCREEN_W, SaveType};

pub const FB_ABI_VERSION: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FbStatus {
    Ok = 0,
    ErrArg = 1,
    ErrRom = 2,
    ErrBios = 3,
    ErrState = 4,
    ErrSave = 5,
    ErrPanic = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FbInfo {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub sample_rate: u32,
    pub ports: u32,
}

/// Bytes of the RTC record `Gba::save_data` appends on clock carts.
const RTC_TRAILER_LEN: usize = 16;
const FB_BYTES: usize = SCREEN_W * SCREEN_H * 4;

/// The opaque `FbCore*` handed to the app.
pub struct FbCore {
    gba: Gba,
    /// LLE image to boot from again on `reset` (None: HLE).
    bios: Option<Vec<u8>>,
    rgba: Box<[u8]>,
    audio: Audio,
    scratch: Vec<[i16; 2]>,
    keys: u16,
    sun: u8,
    /// Save memory changed since the last complete `sram_read`.
    sram_dirty: Cell<bool>,
    poisoned: Cell<bool>,
}

impl FbCore {
    fn new(rom: Vec<u8>, bios: Option<Vec<u8>>) -> Result<Self, FbStatus> {
        let opts = Options { bios: bios.clone().map_or(BiosMode::Hle, BiosMode::Lle), save_type: None };
        let gba = Gba::new(rom, opts).map_err(|e| match e {
            gba_core::CoreError::BadBios(_) => FbStatus::ErrBios,
            _ => FbStatus::ErrRom,
        })?;
        let mut core = FbCore {
            gba,
            bios,
            rgba: vec![0; FB_BYTES].into_boxed_slice(),
            audio: Audio::new(),
            scratch: Vec::with_capacity(1024),
            keys: 0,
            sun: 0,
            sram_dirty: Cell::new(false),
            poisoned: Cell::new(false),
        };
        core.convert_picture();
        Ok(core)
    }

    fn convert_picture(&mut self) {
        let lut = rgba_lut();
        for (dst, &p) in self.rgba.as_chunks_mut::<4>().0.iter_mut().zip(self.gba.framebuffer().iter()) {
            dst.copy_from_slice(&lut[(p & 0x7FFF) as usize]);
        }
    }

    fn run_frame(&mut self) {
        self.gba.run_frame();
        self.convert_picture();
        self.gba.drain_audio(&mut self.scratch);
        let src_rate = CLOCK_HZ as f64 / self.gba.bus.apu.cycles_per_sample() as f64;
        self.audio.push(&self.scratch, src_rate);
        self.scratch.clear();
        if self.gba.save_dirty() {
            self.sram_dirty.set(true);
        }
    }

    /// Power cycle: a fresh machine on the same ROM and BIOS. Battery save, clock, sun level and
    /// held keys carry over, as they would on hardware.
    fn reset(&mut self) {
        let save = self.gba.save_data();
        let rtc = self.gba.rtc_time();
        let rom = self.gba.bus.cart.take_rom();
        let opts = Options { bios: self.bios.clone().map_or(BiosMode::Hle, BiosMode::Lle), save_type: None };
        self.gba = Gba::new(rom, opts).expect("the ROM and BIOS were accepted at create");
        // A save still equal to the fresh one was never written; loading it anyway would fix an
        // EEPROM's size before the game's first access does.
        if let Some(save) = save
            && self.gba.save_data().as_ref() != Some(&save)
        {
            self.gba.load_save(&save);
        }
        if let Some(t) = rtc {
            self.gba.set_rtc_clock(t);
        }
        self.gba.set_sun_level(self.sun);
        self.gba.set_keys(Keys(self.keys));
        self.audio = Audio::new();
    }

    /// Lengths `sram_write` accepts: the backend's size, plus the RTC record on clock carts.
    fn save_len_ok(&self, len: usize) -> bool {
        let sizes: &[usize] = match self.gba.bus.cart.save_type {
            SaveType::None => return false,
            SaveType::Sram32K => &[SRAM_SIZE],
            SaveType::Flash64K => &[0x10000],
            SaveType::Flash128K => &[0x20000],
            SaveType::Eeprom => &[0x200, 0x2000],
        };
        sizes.contains(&len) || (self.gba.has_rtc() && sizes.iter().any(|&s| s + RTC_TRAILER_LEN == len))
    }
}

/// BGR555 -> R,G,B,255, indexed by `texel & 0x7FFF`. 5-bit channels widen as `c << 3 | c >> 2`.
fn rgba_lut() -> &'static [[u8; 4]] {
    static LUT: OnceLock<Vec<[u8; 4]>> = OnceLock::new();
    LUT.get_or_init(|| {
        (0..0x8000u16)
            .map(|p| {
                let w = |c: u16| ((c << 3) | (c >> 2)) as u8;
                [w(p & 0x1F), w((p >> 5) & 0x1F), w((p >> 10) & 0x1F), 255]
            })
            .collect()
    })
}

/// The cartridge header checks the BIOS makes before booting (complement checksum, fixed byte),
/// relaxed: a ROM is refused only when both fail.
fn header_ok(rom: &[u8]) -> bool {
    if rom.len() < 0xC0 {
        return false;
    }
    let sum = rom[0xA0..0xBD].iter().fold(0u8, |c, &b| c.wrapping_sub(b)).wrapping_sub(0x19);
    rom[0xB2] == 0x96 || rom[0xBD] == sum
}

// ---------------------------------------------------------------------------------------------
// Call guards
// ---------------------------------------------------------------------------------------------

/// Run `f` on a live core. NULL yields `null`, a poisoned core `poisoned`; a panic poisons it.
fn with_mut<R>(core: *mut FbCore, null: R, poisoned: R, f: impl FnOnce(&mut FbCore) -> R) -> R {
    // SAFETY: the app passes NULL or a pointer from `fb_gba_create`, not yet destroyed, used from
    // one thread at a time (CORE-ABI "Rules for every function").
    let Some(c) = (unsafe { core.as_mut() }) else { return null };
    if c.poisoned.get() {
        return poisoned;
    }
    match catch_unwind(AssertUnwindSafe(|| f(&mut *c))) {
        Ok(r) => r,
        Err(_) => {
            c.poisoned.set(true);
            poisoned
        }
    }
}

/// `with_mut` for the `const FbCore*` entry points.
fn with_ref<R>(core: *const FbCore, null: R, poisoned: R, f: impl FnOnce(&FbCore) -> R) -> R {
    // SAFETY: as in `with_mut`.
    let Some(c) = (unsafe { core.as_ref() }) else { return null };
    if c.poisoned.get() {
        return poisoned;
    }
    match catch_unwind(AssertUnwindSafe(|| f(c))) {
        Ok(r) => r,
        Err(_) => {
            c.poisoned.set(true);
            poisoned
        }
    }
}

/// Borrow a caller buffer. NULL is only valid with length 0.
///
/// # Safety
/// `ptr` must be NULL or valid for `len` bytes for the duration of the call.
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return (len == 0).then_some(&[]);
    }
    Some(unsafe { std::slice::from_raw_parts(ptr, len) })
}

// ---------------------------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_abi_version() -> u32 {
    FB_ABI_VERSION
}

/// Create a core from a ROM image (copied). `bios`/`bios_len` = NULL/0 boots with the HLE BIOS.
/// On failure returns NULL and writes the status to `*out` when `out` is not NULL.
///
/// # Safety
/// `rom` must be valid for `rom_len` bytes, `bios` for `bios_len` bytes (or NULL), `out` NULL or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_create(
    rom: *const u8,
    rom_len: usize,
    bios: *const u8,
    bios_len: usize,
    out: *mut FbStatus,
) -> *mut FbCore {
    let result = catch_unwind(|| {
        let rom = match unsafe { bytes(rom, rom_len) } {
            Some(r) if !r.is_empty() => r,
            _ => return Err(FbStatus::ErrArg),
        };
        let Some(bios) = (unsafe { bytes(bios, bios_len) }) else { return Err(FbStatus::ErrArg) };
        if rom.len() > ROM_MAX || !header_ok(rom) {
            return Err(FbStatus::ErrRom);
        }
        let bios = (!bios.is_empty()).then(|| bios.to_vec());
        FbCore::new(rom.to_vec(), bios)
    })
    .unwrap_or(Err(FbStatus::ErrPanic));
    let (ptr, status) = match result {
        Ok(core) => (Box::into_raw(Box::new(core)), FbStatus::Ok),
        Err(s) => (std::ptr::null_mut(), s),
    };
    if !out.is_null() {
        // SAFETY: the caller passes NULL or a writable FbStatus.
        unsafe { out.write(status) };
    }
    ptr
}

/// Free a core. NULL is a no-op.
///
/// # Safety
/// `core` must be NULL or a live pointer from `fb_gba_create`; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_destroy(core: *mut FbCore) {
    if core.is_null() {
        return;
    }
    // SAFETY: created by Box::into_raw in fb_gba_create.
    let boxed = unsafe { Box::from_raw(core) };
    let _ = catch_unwind(AssertUnwindSafe(move || drop(boxed)));
}

#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_info(core: *const FbCore) -> FbInfo {
    let info = FbInfo {
        width: SCREEN_W as u32,
        height: SCREEN_H as u32,
        fps_num: CLOCK_HZ as u32,
        fps_den: CYCLES_PER_FRAME as u32,
        sample_rate: OUT_RATE,
        ports: 1,
    };
    with_ref(core, FbInfo::default(), FbInfo::default(), |_| info)
}

#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_reset(core: *mut FbCore) -> FbStatus {
    with_mut(core, FbStatus::ErrArg, FbStatus::ErrPanic, |c| {
        c.reset();
        FbStatus::Ok
    })
}

/// Run to the start of the next V-Blank, then convert the picture and resample the audio.
#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_run_frame(core: *mut FbCore) -> FbStatus {
    with_mut(core, FbStatus::ErrArg, FbStatus::ErrPanic, |c| {
        c.run_frame();
        FbStatus::Ok
    })
}

/// 240x160 RGBA8888 (A = 255), stride 960. Valid until the next run_frame, state_load, reset or
/// destroy. NULL for a NULL or poisoned core.
#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_framebuffer(core: *const FbCore) -> *const u8 {
    with_ref(core, std::ptr::null(), std::ptr::null(), |c| c.rgba.as_ptr())
}

/// Move up to `max_frames` 48 kHz stereo frames (interleaved L,R) into `out`. Returns frames written.
///
/// # Safety
/// `out` must be NULL or writable for `max_frames * 2` i16s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_audio_read(core: *mut FbCore, out: *mut i16, max_frames: usize) -> usize {
    if out.is_null() {
        return 0;
    }
    with_mut(core, 0, 0, |c| {
        let n = max_frames.min(c.audio.len());
        // SAFETY: the caller guarantees max_frames * 2 writable i16s; n <= max_frames.
        let dst = unsafe { std::slice::from_raw_parts_mut(out, n * 2) };
        c.audio.read(dst)
    })
}

/// Held buttons, CORE-ABI bit order (= GBA KEYINPUT order, active high). Port 0 only.
#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_set_input(core: *mut FbCore, port: u32, mask: u32) {
    if port != 0 {
        return;
    }
    with_mut(core, (), (), |c| {
        c.keys = (mask & Keys::ALL as u32) as u16;
        c.gba.set_keys(Keys(c.keys));
    })
}

/// Battery save size in bytes; 0 without one. Includes the 16-byte RTC record on clock carts.
#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_sram_size(core: *const FbCore) -> usize {
    with_ref(core, 0, 0, |c| c.gba.save_data().map_or(0, |d| d.len()))
}

/// Copy the battery save into `out`. Returns bytes copied; a complete copy clears the dirty flag.
///
/// # Safety
/// `out` must be NULL or writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_sram_read(core: *const FbCore, out: *mut u8, cap: usize) -> usize {
    if out.is_null() {
        return 0;
    }
    with_ref(core, 0, 0, |c| {
        let Some(data) = c.gba.save_data() else { return 0 };
        let n = data.len().min(cap);
        // SAFETY: the caller guarantees cap writable bytes; n <= cap.
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), out, n) };
        if n == data.len() {
            c.sram_dirty.set(false);
        }
        n
    })
}

/// Load a battery save. `FB_ERR_SAVE` without save memory or for a size the cartridge can't hold.
///
/// # Safety
/// `data` must be valid for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_sram_write(core: *mut FbCore, data: *const u8, len: usize) -> FbStatus {
    let Some(data) = (unsafe { bytes(data, len) }) else { return FbStatus::ErrArg };
    with_mut(core, FbStatus::ErrArg, FbStatus::ErrPanic, |c| {
        if !c.save_len_ok(data.len()) {
            return FbStatus::ErrSave;
        }
        c.gba.load_save(data);
        c.sram_dirty.set(false);
        FbStatus::Ok
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_sram_dirty(core: *const FbCore) -> bool {
    with_ref(core, false, false, |c| c.sram_dirty.get())
}

/// Serialize the machine (gba-core's versioned "GBAS" format, without the ROM). Free the buffer
/// with `fb_gba_buffer_free`.
///
/// # Safety
/// `out` and `out_len` must be writable (NULL yields `FB_ERR_ARG`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_state_save(core: *mut FbCore, out: *mut *mut u8, out_len: *mut usize) -> FbStatus {
    if out.is_null() || out_len.is_null() {
        return FbStatus::ErrArg;
    }
    with_mut(core, FbStatus::ErrArg, FbStatus::ErrPanic, |c| {
        let buf = c.gba.save_state().into_boxed_slice();
        let len = buf.len();
        // SAFETY: both checked non-NULL above; the caller guarantees they are writable.
        unsafe {
            out.write(Box::into_raw(buf).cast::<u8>());
            out_len.write(len);
        }
        FbStatus::Ok
    })
}

/// Restore a state from `fb_gba_state_save`. Anything else (another core's state, another
/// version, corrupt bytes) yields `FB_ERR_STATE` and leaves the machine as it was.
///
/// # Safety
/// `data` must be valid for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_state_load(core: *mut FbCore, data: *const u8, len: usize) -> FbStatus {
    let Some(data) = (unsafe { bytes(data, len) }) else { return FbStatus::ErrArg };
    with_mut(core, FbStatus::ErrArg, FbStatus::ErrPanic, |c| {
        let before = c.gba.save_data();
        if c.gba.load_state(data).is_err() {
            return FbStatus::ErrState;
        }
        // The state carries its own save memory: if that differs, the app has something to write.
        if c.gba.save_data() != before {
            c.sram_dirty.set(true);
        }
        c.gba.set_keys(Keys(c.keys));
        FbStatus::Ok
    })
}

/// Free a buffer from `fb_gba_state_save`. NULL is a no-op.
///
/// # Safety
/// `ptr`/`len` must be exactly what `fb_gba_state_save` returned, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fb_gba_buffer_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: from Box::<[u8]>::into_raw with this length in fb_gba_state_save.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) });
}

/// Seed the cartridge clock with local wall-clock time (seconds since 1970-01-01, local). No-op
/// without an RTC. Call after create and after state_load.
#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_set_rtc(core: *mut FbCore, local_unix_secs: i64) {
    with_mut(core, (), (), |c| c.gba.set_rtc_clock(local_unix_secs))
}

/// Light on Boktai's solar sensor, 0 (dark) to 10 (clamped). No-op without the sensor.
#[unsafe(no_mangle)]
pub extern "C" fn fb_gba_set_sun_level(core: *mut FbCore, level: u8) {
    with_mut(core, (), (), |c| {
        c.sun = level.min(gba_core::SUN_MAX);
        c.gba.set_sun_level(c.sun);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ROM with a valid header: `b .` at the entry point.
    fn tiny_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x4000];
        rom[0..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes());
        rom[0xA0..0xAC].copy_from_slice(b"FFI TEST    ");
        rom[0xB2] = 0x96;
        rom[0xBD] = rom[0xA0..0xBD].iter().fold(0u8, |c, &b| c.wrapping_sub(b)).wrapping_sub(0x19);
        rom
    }

    fn create(rom: &[u8]) -> *mut FbCore {
        let mut st = FbStatus::ErrPanic;
        let c = unsafe { fb_gba_create(rom.as_ptr(), rom.len(), std::ptr::null(), 0, &mut st) };
        assert_eq!(st, FbStatus::Ok);
        c
    }

    #[test]
    fn lut_widens_channels() {
        let lut = rgba_lut();
        assert_eq!(lut[0], [0, 0, 0, 255]);
        assert_eq!(lut[0x7FFF], [255, 255, 255, 255]);
        assert_eq!(lut[0x001F], [255, 0, 0, 255]);
        assert_eq!(lut[0x03E0], [0, 255, 0, 255]);
        assert_eq!(lut[0x7C00], [0, 0, 255, 255]);
        assert_eq!(lut[0x0010], [0x84, 0, 0, 255]);
    }

    #[test]
    fn header_check() {
        let rom = tiny_rom();
        assert!(header_ok(&rom));
        let mut no_fix = rom.clone();
        no_fix[0xBD] ^= 0x5A; // never run through gbafix
        assert!(header_ok(&no_fix));
        let mut no_96 = rom.clone();
        no_96[0xB2] = 0;
        no_96[0xBD] = no_96[0xBD].wrapping_add(0x96); // the checksum covers 0xB2
        assert!(header_ok(&no_96));
        let mut bad = no_96;
        bad[0xBD] ^= 0x5A;
        assert!(!header_ok(&bad));
        assert!(!header_ok(&rom[..0xBF]));
    }

    #[test]
    fn panic_poisons_until_destroy() {
        let core = create(&tiny_rom());
        let r = with_mut(core, FbStatus::ErrArg, FbStatus::ErrPanic, |_| -> FbStatus { panic!("injected") });
        assert_eq!(r, FbStatus::ErrPanic);
        assert_eq!(fb_gba_run_frame(core), FbStatus::ErrPanic);
        assert_eq!(fb_gba_reset(core), FbStatus::ErrPanic);
        assert!(fb_gba_framebuffer(core).is_null());
        assert_eq!(fb_gba_info(core), FbInfo::default());
        let mut out = std::ptr::null_mut();
        let mut len = 0;
        assert_eq!(unsafe { fb_gba_state_save(core, &mut out, &mut len) }, FbStatus::ErrPanic);
        assert!(out.is_null());
        unsafe { fb_gba_destroy(core) };
    }

    #[test]
    fn unfixed_checksum_still_creates() {
        let mut rom = tiny_rom();
        rom[0xBD] = rom[0xBD].wrapping_add(1);
        let core = create(&rom);
        assert_eq!(fb_gba_run_frame(core), FbStatus::Ok);
        unsafe { fb_gba_destroy(core) };
    }

    #[test]
    fn reset_keeps_running() {
        let core = create(&tiny_rom());
        for _ in 0..3 {
            assert_eq!(fb_gba_run_frame(core), FbStatus::Ok);
        }
        assert_eq!(fb_gba_reset(core), FbStatus::Ok);
        assert_eq!(fb_gba_run_frame(core), FbStatus::Ok);
        unsafe { fb_gba_destroy(core) };
    }
}
