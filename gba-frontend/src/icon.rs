//! The window icon at runtime, from the app's [`Icon`]. Windows loads a `.ico` from disk, so
//! the embedded file is written to the cache folder first; X11 takes `_NET_WM_ICON` ARGB
//! buffers; Wayland and macOS take nothing at runtime (minifb's `set_icon` panics on both, so
//! they are detected and skipped). The exe's own icon on Windows is a resource embedded by
//! `build.rs`.

#![allow(dead_code)]

use crate::ui::mark::{Sprite, SPRITE};
use std::path::Path;

/// An app's window icon.
pub struct Icon {
    /// The `.ico` (Windows), written as `<cache dir>/<ico_name>`.
    pub ico: &'static [u8],
    pub ico_name: &'static str,
    /// PNGs for X11's `_NET_WM_ICON`, smallest first.
    pub pngs: &'static [&'static [u8]],
    /// The mark, drawn at 1x-3x for X11 when no PNG decodes.
    pub sprite: fn() -> &'static Sprite,
}

/// Give `window` the icon where the platform allows it.
#[cfg(windows)]
pub fn install(window: &mut minifb::Window, cache_dir: Option<&Path>, icon: &Icon) {
    use std::os::windows::ffi::OsStrExt;
    let Some(dir) = cache_dir else { return };
    let path = dir.join(icon.ico_name);
    if std::fs::read(&path).ok().as_deref() != Some(icon.ico) {
        if let Err(e) = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, icon.ico)) {
            eprintln!("warning: window icon not written: {e}");
            return;
        }
    }
    // minifb copies the path inside `set_icon` (GetFullPathNameW + LoadImageW). Its own
    // `Icon::from_str` returns a pointer into a local Vec, so the path is built here instead.
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    window.set_icon(minifb::Icon::Path(wide.as_ptr()));
}

/// Give `window` the icon where the platform allows it.
#[cfg(target_os = "linux")]
pub fn install(window: &mut minifb::Window, _cache_dir: Option<&Path>, icon: &Icon) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // minifb tries Wayland before X11, and its Wayland `set_icon` is `unimplemented!()`.
    let x11 = matches!(window.window_handle().map(|h| h.as_raw()), Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)));
    if !x11 {
        return;
    }
    let buf = net_wm_icon(icon);
    // X copies the data inside XChangeProperty.
    window.set_icon(minifb::Icon::Buffer(buf.as_ptr(), buf.len() as u32));
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn install(_window: &mut minifb::Window, _cache_dir: Option<&Path>, _icon: &Icon) {}

/// `_NET_WM_ICON` data: `[w, h, argb...]` per size, concatenated. The icon's PNGs, or the
/// sprite at 1x/2x/3x if none decodes.
pub fn net_wm_icon(icon: &Icon) -> Vec<u64> {
    let mut buf: Vec<u64> = Vec::new();
    for png in icon.pngs {
        if let Some((w, h, px)) = decode_argb(png) {
            buf.push(w as u64);
            buf.push(h as u64);
            buf.extend(px.into_iter().map(u64::from));
        }
    }
    if buf.is_empty() {
        let argb = (icon.sprite)().argb_paper();
        let n = SPRITE;
        for s in 1..=3usize {
            buf.push((n * s) as u64);
            buf.push((n * s) as u64);
            for y in 0..n * s {
                for x in 0..n * s {
                    buf.push(argb[(y / s) * n + x / s] as u64);
                }
            }
        }
    }
    buf
}

/// PNG bytes -> (w, h, 0xAARRGGBB per pixel); 8-bit RGB or RGBA only.
pub fn decode_argb(bytes: &[u8]) -> Option<(u32, u32, Vec<u32>)> {
    let mut reader = png::Decoder::new(bytes).read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let px = &buf[..info.buffer_size()];
    let argb: Vec<u32> = match info.color_type {
        png::ColorType::Rgba => px.chunks_exact(4).map(|p| (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32).collect(),
        png::ColorType::Rgb => px.chunks_exact(3).map(|p| 0xFF00_0000 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32).collect(),
        _ => return None,
    };
    Some((info.width, info.height, argb))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(rgba).unwrap();
        out
    }

    fn blank() -> &'static Sprite {
        static S: std::sync::OnceLock<Sprite> = std::sync::OnceLock::new();
        S.get_or_init(|| crate::ui::mark::parse_mark("<svg/>"))
    }

    #[test]
    fn pngs_decode_and_a_sprite_stands_in_without_them() {
        let one = png(1, 1, &[0x10, 0x20, 0x30, 0xFF]);
        assert_eq!(decode_argb(&one), Some((1, 1, vec![0xFF10_2030])));
        assert_eq!(decode_argb(b"not a png"), None);
        let pngs: &'static [&'static [u8]] = Box::leak(vec![&*Box::leak(one.into_boxed_slice())].into_boxed_slice());
        let icon = Icon { ico: &[], ico_name: "x.ico", pngs, sprite: blank };
        assert_eq!(net_wm_icon(&icon), vec![1, 1, 0xFF10_2030]);
        let fallback = Icon { ico: &[], ico_name: "x.ico", pngs: &[b"junk"], sprite: blank };
        let buf = net_wm_icon(&fallback);
        assert_eq!(buf.len(), (2 + 256) + (2 + 1024) + (2 + 2304), "16, 32 and 48 px");
        assert!(buf[2..258].iter().all(|p| *p == 0), "a blank sprite is clear");
    }
}
