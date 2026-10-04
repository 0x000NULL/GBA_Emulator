//! minifb window: resizable, presents either a window-resolution canvas (menus) or the game
//! frame on a letterbox (fast path: BGR555 -> 0RGB through a LUT, integer upscale), and gathers
//! the per-frame input snapshot.

use crate::config::{SCREEN_H, SCREEN_W};
use crate::ui::input::{actions, KeyMap, PadRepeat, UiInput};
use basket_app::pads::PadPoll;
use crate::ui::{Canvas, Rgb};
use anyhow::{anyhow, Result};
use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, Window, WindowOptions};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Nearest-neighbour upscale of a 240x160 BGR555 framebuffer into `out` (240*scale x 160*scale, 0RGB).
#[cfg_attr(not(test), allow(dead_code))]
pub fn scale_into(fb: &[u16], lut: &[u32], scale: usize, out: &mut [u32]) {
    scale_into_at(fb, lut, scale, out, SCREEN_W * scale, 0, 0);
}

/// Upscale into a larger `out` buffer of width `out_w`, placing the picture at (x, y).
/// Rows and columns that fall outside `out` are skipped.
pub fn scale_into_at(fb: &[u16], lut: &[u32], scale: usize, out: &mut [u32], out_w: usize, x: i32, y: i32) {
    debug_assert_eq!(fb.len(), SCREEN_W * SCREEN_H);
    let out_h = out.len() / out_w.max(1);
    let dst_w = SCREEN_W * scale;
    for sy in 0..SCREEN_H {
        let src = &fb[sy * SCREEN_W..(sy + 1) * SCREEN_W];
        let dy0 = y + (sy * scale) as i32;
        // the first scaled row is filled from the LUT, the rest are copies of it
        let mut first: Option<usize> = None;
        for r in 0..scale {
            let dy = dy0 + r as i32;
            if dy < 0 || dy >= out_h as i32 {
                continue;
            }
            let row0 = dy as usize * out_w;
            match first {
                Some(src_row) => {
                    let x0 = x.max(0) as usize;
                    let x1 = ((x + dst_w as i32).min(out_w as i32)).max(0) as usize;
                    if x1 > x0 {
                        out.copy_within(src_row + x0..src_row + x1, row0 + x0);
                    }
                }
                None => {
                    for (sx, &p) in src.iter().enumerate() {
                        let c = lut[(p & 0x7FFF) as usize];
                        let px0 = x + (sx * scale) as i32;
                        for s in 0..scale {
                            let dx = px0 + s as i32;
                            if dx >= 0 && dx < out_w as i32 {
                                out[row0 + dx as usize] = c;
                            }
                        }
                    }
                    first = Some(row0);
                }
            }
        }
    }
}

/// Post-processing on the upscaled picture (never in the core).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    None,
    /// Darkens the last row and column of every scaled dot: the gaps of an LCD grid.
    LcdGrid,
    /// Averages with the previous presented picture: the slow original screen.
    FrameBlend,
}

/// LCD grid: darken every `scale`-th row and column of the `fw` x `fh` picture at (x, y).
pub fn lcd_grid(out: &mut [u32], out_w: usize, x: i32, y: i32, scale: usize, fw: usize, fh: usize) {
    if scale < 2 {
        return;
    }
    let out_h = out.len() / out_w.max(1);
    let darken = |p: u32| -> u32 {
        let r = ((p >> 16) & 0xFF) * 3 / 5;
        let g = ((p >> 8) & 0xFF) * 3 / 5;
        let b = (p & 0xFF) * 3 / 5;
        r << 16 | g << 8 | b
    };
    for row in 0..fh * scale {
        let dy = y + row as i32;
        if dy < 0 || dy >= out_h as i32 {
            continue;
        }
        let base = dy as usize * out_w;
        let grid_row = row % scale == scale - 1;
        for col in 0..fw * scale {
            let dx = x + col as i32;
            if dx < 0 || dx >= out_w as i32 {
                continue;
            }
            if grid_row || col % scale == scale - 1 {
                out[base + dx as usize] = darken(out[base + dx as usize]);
            }
        }
    }
}

/// Frame blend: `out = (out + prev) / 2` per channel, then `prev = out`.
pub fn frame_blend(out: &mut [u32], prev: &mut Vec<u32>) {
    if prev.len() != out.len() {
        prev.clear();
        prev.extend_from_slice(out);
        return;
    }
    for (o, p) in out.iter_mut().zip(prev.iter_mut()) {
        let cur = *o;
        let a = ((cur ^ *p) & 0x00FE_FEFE) >> 1;
        let b = cur & *p;
        *o = a.wrapping_add(b);
        *p = cur;
    }
}

fn pack(c: Rgb) -> u32 {
    (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

struct CharSink(Arc<Mutex<Vec<char>>>);

impl InputCallback for CharSink {
    fn add_char(&mut self, uni_char: u32) {
        if let Some(c) = char::from_u32(uni_char) {
            if !c.is_control() {
                self.0.lock().unwrap().push(c);
            }
        }
    }
}

/// The primary display's size in the units windows are created in, where it can be read.
pub fn display_size() -> Option<(usize, usize)> {
    display::size()
}

#[cfg(windows)]
mod display {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetSystemMetrics(index: i32) -> i32;
    }

    pub fn size() -> Option<(usize, usize)> {
        // SM_CXSCREEN / SM_CYSCREEN: the primary monitor, in this (DPI-unaware) process's units
        let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
        (w > 0 && h > 0).then_some((w as usize, h as usize))
    }
}

#[cfg(target_os = "macos")]
mod display {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayPixelsWide(display: u32) -> usize;
        fn CGDisplayPixelsHigh(display: u32) -> usize;
    }

    pub fn size() -> Option<(usize, usize)> {
        let (w, h) = unsafe {
            let d = CGMainDisplayID();
            (CGDisplayPixelsWide(d), CGDisplayPixelsHigh(d))
        };
        (w > 0 && h > 0).then_some((w, h))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod display {
    /// The first connected DRM connector's preferred mode: works under X11 and Wayland without
    /// linking either.
    pub fn size() -> Option<(usize, usize)> {
        let mut connectors: Vec<_> = std::fs::read_dir("/sys/class/drm").ok()?.flatten().map(|e| e.path()).collect();
        connectors.sort();
        connectors.iter().find_map(|c| {
            let status = std::fs::read_to_string(c.join("status")).ok()?;
            if status.trim() != "connected" {
                return None;
            }
            super::parse_mode(std::fs::read_to_string(c.join("modes")).ok()?.lines().next()?)
        })
    }
}

/// `1920x1080` (a DRM mode line; some carry a suffix such as `i`).
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
fn parse_mode(line: &str) -> Option<(usize, usize)> {
    let (w, h) = line.trim().split_once('x')?;
    let h: String = h.chars().take_while(|c| c.is_ascii_digit()).collect();
    let (w, h) = (w.parse().ok()?, h.parse().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

pub struct Video {
    window: Window,
    buf: Vec<u32>,
    lut: &'static [u32],
    chars: Arc<Mutex<Vec<char>>>,
    pad_repeat: PadRepeat,
    mouse_was_down: bool,
    pub filter: Filter,
    prev: Vec<u32>,
}

impl Video {
    /// Open a resizable window of `w` x `h` client pixels.
    pub fn new(title: &str, w: usize, h: usize) -> Result<Self> {
        let opts = WindowOptions { resize: true, scale: Scale::X1, scale_mode: ScaleMode::Stretch, ..WindowOptions::default() };
        Self::open(title, w, h, opts)
    }

    /// `--fullscreen`: a borderless window covering the primary display (minifb has no exclusive
    /// fullscreen). None when the display size cannot be read here.
    pub fn new_fullscreen(title: &str) -> Result<Option<Self>> {
        let Some((w, h)) = display_size() else { return Ok(None) };
        let opts = WindowOptions { borderless: true, none: true, resize: false, scale: Scale::X1, scale_mode: ScaleMode::Stretch, ..WindowOptions::default() };
        let mut video = Self::open(title, w, h, opts)?;
        video.window.set_position(0, 0);
        Ok(Some(video))
    }

    fn open(title: &str, w: usize, h: usize, opts: WindowOptions) -> Result<Self> {
        let mut window = Window::new(title, w, h, opts).map_err(|e| anyhow!("creating window: {e}"))?;
        // minifb rate-limits update() to 250 fps by default; pacing is ours.
        window.set_target_fps(0);
        let chars = Arc::new(Mutex::new(Vec::new()));
        window.set_input_callback(Box::new(CharSink(chars.clone())));
        Ok(Video { window, buf: vec![0; w * h], lut: crate::pixels::bgr555_lut(), chars, pad_repeat: PadRepeat::new(&crate::input::GBA_BUTTONS), mouse_was_down: false, filter: Filter::None, prev: Vec::new() })
    }

    /// The Strawberry icon where the platform allows it (`icon.rs`); `cache_dir` holds the
    /// `.ico` Windows loads from disk.
    pub fn set_brand_icon(&mut self, cache_dir: Option<&std::path::Path>) {
        crate::icon::install(&mut self.window, cache_dir, &crate::brand::ICON);
    }

    /// Current client size.
    pub fn size(&self) -> (usize, usize) {
        let (w, h) = self.window.get_size();
        (w.max(1), h.max(1))
    }

    fn ensure_buf(&mut self) -> (usize, usize) {
        let (w, h) = self.size();
        if self.buf.len() != w * h {
            self.buf = vec![0; w * h];
        }
        (w, h)
    }

    /// Present the game frame at `scale` on a letterbox of colour `bg`; the frame is centred.
    pub fn present_game(&mut self, fb: &[u16], scale: u32, bg: Rgb) -> Result<()> {
        let (w, h) = self.ensure_buf();
        let scale = scale.max(1) as usize;
        let (x, y) = crate::config::SCREEN.centred(w as u32, h as u32, scale as u32);
        self.buf.fill(pack(bg));
        scale_into_at(fb, self.lut, scale, &mut self.buf, w, x, y);
        match self.filter {
            Filter::None => {}
            Filter::LcdGrid => lcd_grid(&mut self.buf, w, x, y, scale, SCREEN_W, SCREEN_H),
            Filter::FrameBlend => frame_blend(&mut self.buf, &mut self.prev),
        }
        self.window.update_with_buffer(&self.buf, w, h).map_err(|e| anyhow!("presenting frame: {e}"))
    }

    /// Present a window-resolution canvas (it is resized to the window first by the caller).
    pub fn present_canvas(&mut self, canvas: &Canvas) -> Result<()> {
        let (w, h) = self.ensure_buf();
        canvas.to_0rgb(&mut self.buf);
        if self.buf.len() != w * h {
            // the window changed size between layout and present; draw next frame instead
            self.window.update();
            return Ok(());
        }
        self.window.update_with_buffer(&self.buf, w, h).map_err(|e| anyhow!("presenting canvas: {e}"))
    }

    /// Pump window events without redrawing.
    pub fn poll(&mut self) {
        self.window.update();
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    /// Everything that happened since the last update, as the screens want it.
    pub fn gather_input(&mut self, map: &KeyMap, pad: PadPoll, now: Instant) -> UiInput {
        let pressed = self.window.get_keys_pressed(KeyRepeat::No);
        let repeated = self.window.get_keys_pressed(KeyRepeat::Yes);
        let down = self.window.get_keys();
        let chars: Vec<char> = std::mem::take(&mut *self.chars.lock().unwrap());
        let mouse = self.window.get_mouse_pos(MouseMode::Pass).unwrap_or((-1.0, -1.0));
        let mouse_down = self.window.get_mouse_down(MouseButton::Left);
        let clicked = mouse_down && !self.mouse_was_down;
        self.mouse_was_down = mouse_down;
        let wheel = self.window.get_scroll_wheel().map(|(_, y)| y).unwrap_or(0.0);
        let (pad_pressed, pad_repeated) = self.pad_repeat.update(pad.mask, now);
        let acts = actions(map, &pressed, &repeated, pad_pressed, pad_repeated);
        let game_mask = crate::input::sanitize(map.mask(|k| down.contains(&k)) | pad.mask);
        UiInput { pressed, repeated, down, chars, mouse, mouse_down, clicked, wheel, actions: acts, pad_buttons: pad.pressed, game_mask }
    }

    pub fn is_key_down(&self, k: Key) -> bool {
        self.window.is_key_down(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drm_mode_lines() {
        assert_eq!(parse_mode("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_mode("3840x2160
"), Some((3840, 2160)));
        assert_eq!(parse_mode("1920x1080i"), Some((1920, 1080)));
        assert_eq!(parse_mode("0x0"), None);
        assert_eq!(parse_mode("garbage"), None);
    }

    #[test]
    fn lut_endpoints() {
        let lut = crate::pixels::bgr555_lut();
        assert_eq!(lut.len(), 0x8000);
        assert_eq!(lut[0], 0x000000);
        assert_eq!(lut[0x7FFF], 0xFFFFFF);
        assert_eq!(lut[0x001F], 0xFF0000, "pure red is the low 5 bits");
        assert_eq!(lut[0x03E0], 0x00FF00);
        assert_eq!(lut[0x7C00], 0x0000FF);
    }

    #[test]
    fn scale_replicates_pixels() {
        let lut = crate::pixels::bgr555_lut();
        let mut fb = vec![0u16; SCREEN_W * SCREEN_H];
        fb[0] = 0x001F; // red at (0,0)
        fb[SCREEN_W + 1] = 0x7C00; // blue at (1,1)
        let scale = 3;
        let w = SCREEN_W * scale;
        let mut out = vec![0u32; w * SCREEN_H * scale];
        scale_into(&fb, lut, scale, &mut out);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(out[y * w + x], 0xFF0000);
            }
        }
        for y in 3..6 {
            for x in 3..6 {
                assert_eq!(out[y * w + x], 0x0000FF);
            }
        }
        assert_eq!(out[3], 0);
        assert_eq!(out[3 * w], 0);
    }

    #[test]
    fn filters() {
        let w = 8;
        let mut out = vec![0xFFFFFFu32; w * 8];
        lcd_grid(&mut out, w, 0, 0, 2, 4, 4);
        assert_eq!(out[0], 0xFFFFFF, "dot interior untouched");
        assert_eq!(out[1], 0x999999, "grid column darkened");
        assert_eq!(out[w], 0x999999, "grid row darkened");
        let mut a = vec![0x000000u32, 0xFFFFFF];
        let mut prev = Vec::new();
        frame_blend(&mut a, &mut prev);
        assert_eq!(a, vec![0x000000, 0xFFFFFF], "first frame passes through");
        let mut b = vec![0xFFFFFFu32, 0xFFFFFF];
        frame_blend(&mut b, &mut prev);
        assert_eq!(b[0], 0x7F7F7F);
        assert_eq!(b[1], 0xFFFFFF);
        assert_eq!(prev, vec![0xFFFFFF, 0xFFFFFF]);
    }

    #[test]
    fn scale_at_offset_into_a_letterbox() {
        let lut = crate::pixels::bgr555_lut();
        let mut fb = vec![0x03E0u16; SCREEN_W * SCREEN_H]; // green
        fb[SCREEN_W * SCREEN_H - 1] = 0x001F; // red bottom-right
        let (w, h) = (1280usize, 800usize);
        let mut out = vec![0x141414u32; w * h];
        scale_into_at(&fb, lut, 4, &mut out, w, 160, 80);
        assert_eq!(out[79 * w + 160], 0x141414, "letterbox above");
        assert_eq!(out[80 * w + 159], 0x141414, "letterbox left");
        assert_eq!(out[80 * w + 160], 0x00FF00);
        assert_eq!(out[(80 + 640 - 1) * w + 160 + 960 - 1], 0xFF0000);
        assert_eq!(out[(80 + 640) * w + 160], 0x141414, "letterbox below");
        // partially off-screen placement does not panic
        let mut small = vec![0u32; 100 * 100];
        scale_into_at(&fb, lut, 2, &mut small, 100, -50, -30);
        assert_eq!(small[0], 0x00FF00);
    }
}
