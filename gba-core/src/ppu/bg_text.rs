//! Text (tile/map) backgrounds: BG0-3 in mode 0, BG0-1 in mode 1.
//! Source: GBATEK "LCD I/O BG Control", "BG Scrolling", "VRAM Character Data", "BG Screen Data Format".

use super::regs::{Mem, TRANSPARENT};
use crate::SCREEN_W;

/// Render one line of text background `bg` into `out`. `y` is the (already mosaic-adjusted)
/// screen line. Tile data addressed at or beyond 0x10000 (OBJ VRAM) reads as transparent.
pub fn render(mem: &Mem, bg: usize, y: usize, out: &mut [u16; SCREEN_W]) {
    let cnt = mem.bgcnt(bg);
    let (hofs, vofs) = mem.bg_scroll(bg);
    let wmask: u32 = if cnt.size & 1 != 0 { 511 } else { 255 };
    let hmask: u32 = if cnt.size & 2 != 0 { 511 } else { 255 };

    let yy = (y as u32 + vofs) & hmask;
    let tile_row = (yy >> 3) & 31;
    let row = (yy & 7) as usize;
    // Screen-block row offset: sizes 2/3 place the lower 256 rows in SC1 (size 2) or SC2/SC3 (size 3).
    let sc_row = if yy >= 256 { if cnt.size == 3 { 2 } else { 1 } } else { 0 };

    let mut x = 0usize;
    while x < SCREEN_W {
        let xx = (x as u32 + hofs) & wmask;
        let tile_col = (xx >> 3) & 31;
        let start_col = (xx & 7) as usize;
        let sc = sc_row + if xx >= 256 { 1 } else { 0 };
        let entry_addr = cnt.screen_base + sc * 0x800 + (tile_row as usize) * 64 + (tile_col as usize) * 2;
        let entry = mem.vram16(entry_addr & 0xFFFF);
        let tile = (entry & 0x3FF) as usize;
        let hflip = entry & (1 << 10) != 0;
        let vflip = entry & (1 << 11) != 0;
        let pal_bank = ((entry >> 12) & 0xF) as usize * 16;
        let r = if vflip { 7 - row } else { row };

        if cnt.bpp8 {
            let row_addr = cnt.char_base + tile * 64 + r * 8;
            for col in start_col..8 {
                if x >= SCREEN_W {
                    break;
                }
                let c = if hflip { 7 - col } else { col };
                let addr = row_addr + c;
                let idx = if addr < 0x10000 { mem.vram[addr] as usize } else { 0 };
                out[x] = if idx == 0 { TRANSPARENT } else { mem.pal_color(idx) };
                x += 1;
            }
        } else {
            let row_addr = cnt.char_base + tile * 32 + r * 4;
            for col in start_col..8 {
                if x >= SCREEN_W {
                    break;
                }
                let c = if hflip { 7 - col } else { col };
                let addr = row_addr + (c >> 1);
                let idx = if addr < 0x10000 {
                    let b = mem.vram[addr];
                    if c & 1 != 0 { (b >> 4) as usize } else { (b & 0xF) as usize }
                } else {
                    0
                };
                out[x] = if idx == 0 { TRANSPARENT } else { mem.pal_color(pal_bank + idx) };
                x += 1;
            }
        }
    }
}
