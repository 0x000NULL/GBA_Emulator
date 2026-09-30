//! Rotation/scaling (affine) backgrounds: BG2 in mode 1, BG2/BG3 in mode 2.
//! Source: GBATEK "LCD I/O BG Rotation/Scaling", "BG Screen Data Format" (rotation/scaling map),
//! "VRAM Overview". 8bpp only; map entries are one byte; area overflow per BGxCNT bit 13.

use super::regs::{Mem, TRANSPARENT};
use crate::SCREEN_W;

/// Render one line of affine background `bg`. `refx`/`refy` are the internal reference point for
/// this line (28-bit signed 8.8 fixed point); `pa`/`pc` are the per-pixel increments.
pub fn render(mem: &Mem, bg: usize, refx: i32, refy: i32, pa: i32, pc: i32, out: &mut [u16; SCREEN_W]) {
    let cnt = mem.bgcnt(bg);
    let size_px: i32 = 128 << cnt.size;
    let tiles_per_row: usize = 16 << cnt.size;
    let mask = size_px - 1;

    let mut fx = refx;
    let mut fy = refy;
    for px in out.iter_mut() {
        let mut ix = fx >> 8;
        let mut iy = fy >> 8;
        fx = fx.wrapping_add(pa);
        fy = fy.wrapping_add(pc);
        if cnt.wrap {
            ix &= mask;
            iy &= mask;
        } else if ix < 0 || iy < 0 || ix >= size_px || iy >= size_px {
            *px = TRANSPARENT;
            continue;
        }
        let (ix, iy) = (ix as usize, iy as usize);
        let map_addr = cnt.screen_base + (iy >> 3) * tiles_per_row + (ix >> 3);
        let tile = mem.vram[map_addr & 0xFFFF] as usize;
        let addr = cnt.char_base + tile * 64 + (iy & 7) * 8 + (ix & 7);
        let idx = if addr < 0x10000 { mem.vram[addr] as usize } else { 0 };
        *px = if idx == 0 { TRANSPARENT } else { mem.pal_color(idx) };
    }
}
