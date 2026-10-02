//! The GBA's pixel format for the generic drawing code: BGR555 through a 32768-entry lookup table
//! (`Canvas::blit_indexed`, the window's fast path). Bit 15 of a texel is never a colour bit: the
//! blits treat it as "transparent" and the table ignores it.

use std::sync::OnceLock;

/// BGR555 -> 0RGB8888, indexed by `texel & 0x7FFF`.
pub fn bgr555_lut() -> &'static [u32] {
    static LUT: OnceLock<Vec<u32>> = OnceLock::new();
    LUT.get_or_init(|| {
        (0..0x8000u32)
            .map(|p| {
                let [r, g, b] = gba_test::bgr555_to_rgb(p as u16);
                (r as u32) << 16 | (g as u32) << 8 | b as u32
            })
            .collect()
    })
}
