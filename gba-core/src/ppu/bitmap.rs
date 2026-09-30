//! Bitmap backgrounds (modes 3, 4, 5), always BG2 with rotation/scaling applied.
//! Source: GBATEK "LCD VRAM Bitmap BG Modes". Outside of the bitmap is always transparent
//! (area overflow bit ignored). Frame select (DISPCNT bit 4) applies to modes 4 and 5 only.

use super::regs::{Mem, TRANSPARENT};
use crate::SCREEN_W;

pub fn render(mem: &Mem, mode: u16, refx: i32, refy: i32, pa: i32, pc: i32, out: &mut [u16; SCREEN_W]) {
    let dispcnt = mem.dispcnt();
    let frame: usize = if mode != 3 && dispcnt & (1 << 4) != 0 { 0xA000 } else { 0 };
    let (w, h): (i32, i32) = if mode == 5 { (160, 128) } else { (240, 160) };

    let mut fx = refx;
    let mut fy = refy;
    for px in out.iter_mut() {
        let ix = fx >> 8;
        let iy = fy >> 8;
        fx = fx.wrapping_add(pa);
        fy = fy.wrapping_add(pc);
        if ix < 0 || iy < 0 || ix >= w || iy >= h {
            *px = TRANSPARENT;
            continue;
        }
        let (ix, iy) = (ix as usize, iy as usize);
        *px = match mode {
            3 => mem.vram16((iy * 240 + ix) * 2) & 0x7FFF,
            4 => {
                let idx = mem.vram[frame + iy * 240 + ix] as usize;
                if idx == 0 { TRANSPARENT } else { mem.pal_color(idx) }
            }
            _ => mem.vram16(frame + (iy * 160 + ix) * 2) & 0x7FFF,
        };
    }
}
