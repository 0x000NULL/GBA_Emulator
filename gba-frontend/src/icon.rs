//! The window icon at runtime (the mark from `ui::brand`). Windows loads a `.ico` from disk, so
//! the embedded file is written to the cache folder first; X11 takes `_NET_WM_ICON` ARGB
//! buffers; Wayland and macOS take nothing at runtime (minifb's `set_icon` panics on both, so
//! they are detected and skipped). The exe's own icon on Windows is a resource embedded by
//! `build.rs`.

#![allow(dead_code)]

use std::path::Path;

/// Give `window` the Strawberry icon where the platform allows it.
#[cfg(windows)]
pub fn install(window: &mut minifb::Window, cache_dir: Option<&Path>) {
    use std::os::windows::ffi::OsStrExt;
    let Some(dir) = cache_dir else { return };
    let path = dir.join("strawberry.ico");
    if std::fs::read(&path).ok().as_deref() != Some(crate::ui::brand::ICO) {
        if let Err(e) = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, crate::ui::brand::ICO)) {
            eprintln!("warning: window icon not written: {e}");
            return;
        }
    }
    // minifb copies the path inside `set_icon` (GetFullPathNameW + LoadImageW). Its own
    // `Icon::from_str` returns a pointer into a local Vec, so the path is built here instead.
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    window.set_icon(minifb::Icon::Path(wide.as_ptr()));
}

/// Give `window` the Strawberry icon where the platform allows it.
#[cfg(target_os = "linux")]
pub fn install(window: &mut minifb::Window, _cache_dir: Option<&Path>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // minifb tries Wayland before X11, and its Wayland `set_icon` is `unimplemented!()`.
    let x11 = matches!(window.window_handle().map(|h| h.as_raw()), Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)));
    if !x11 {
        return;
    }
    let buf = net_wm_icon();
    // X copies the data inside XChangeProperty.
    window.set_icon(minifb::Icon::Buffer(buf.as_ptr(), buf.len() as u32));
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn install(_window: &mut minifb::Window, _cache_dir: Option<&Path>) {}

/// `_NET_WM_ICON` data: `[w, h, argb...]` per size, concatenated. The 48 and 128 px PNGs, or
/// the sprite at 1x/2x/3x if they do not decode.
pub fn net_wm_icon() -> Vec<u64> {
    let mut buf: Vec<u64> = Vec::new();
    for png in [crate::ui::brand::ICON_48_PNG, crate::ui::brand::ICON_128_PNG] {
        if let Some((w, h, px)) = decode_argb(png) {
            buf.push(w as u64);
            buf.push(h as u64);
            buf.extend(px.into_iter().map(u64::from));
        }
    }
    if buf.is_empty() {
        let argb = crate::ui::brand::mark().argb_paper();
        let n = crate::ui::brand::SPRITE;
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

    #[test]
    fn icon_pngs_decode_and_the_x11_buffer_lists_both_sizes() {
        let (w, h, px) = decode_argb(crate::ui::brand::ICON_48_PNG).expect("48 px icon");
        assert_eq!((w, h, px.len()), (48, 48, 48 * 48));
        assert!(px.iter().any(|p| p >> 24 == 0xFF), "opaque pixels");
        let buf = net_wm_icon();
        assert_eq!((buf[0], buf[1]), (48, 48));
        let next = 2 + 48 * 48;
        assert_eq!((buf[next], buf[next + 1]), (128, 128));
        assert_eq!(buf.len(), next + 2 + 128 * 128);
    }
}
