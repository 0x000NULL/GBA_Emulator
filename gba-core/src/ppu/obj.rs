//! OBJ (sprite) layer: renders the 128 OAM entries for one scanline into a per-pixel buffer
//! holding the top-most OBJ pixel (colour, BG-relative priority, semi-transparency) plus the
//! OBJ-window mask.
//! Source: GBATEK "LCD OBJ - Overview", "OAM Attributes", "OAM Rotation/Scaling Parameters",
//! "VRAM Character (Tile) Mapping", "LCD I/O Mosaic Function".
//!
//! Ordering: a lower priority number wins; at equal priority the lower OAM index wins.
//! Not modelled in M3: the per-line OBJ rendering cycle budget (1210/954 cycles) and the
//! one-line-early OAM evaluation.

use super::regs::{Mem, TRANSPARENT};
use crate::SCREEN_W;

pub struct ObjLine {
    /// BGR555 colour of the top-most OBJ pixel, or `TRANSPARENT`.
    pub color: [u16; SCREEN_W],
    /// Priority relative to BG (0-3) of that pixel; 4 when no pixel.
    pub prio: [u8; SCREEN_W],
    /// That pixel came from a semi-transparent (mode 1) OBJ.
    pub semi: [bool; SCREEN_W],
    /// Pixel is covered by an OBJ-window (mode 2) OBJ.
    pub window: [bool; SCREEN_W],
}

impl ObjLine {
    pub fn new() -> Self {
        ObjLine { color: [TRANSPARENT; SCREEN_W], prio: [4; SCREEN_W], semi: [false; SCREEN_W], window: [false; SCREEN_W] }
    }

    pub fn clear(&mut self) {
        self.color.fill(TRANSPARENT);
        self.prio.fill(4);
        self.semi.fill(false);
        self.window.fill(false);
    }
}

/// (width, height) by [shape][size].
const SIZES: [[(i32, i32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

struct Sprite {
    w: i32,
    tile: usize,
    bpp8: bool,
    pal_bank: usize,
    map_1d: bool,
    bitmap_mode: bool,
}

impl Sprite {
    /// Palette index (0 = transparent) of texel (tx, ty), 0 <= tx < w, 0 <= ty < h.
    #[inline]
    fn texel(&self, mem: &Mem, tx: i32, ty: i32) -> usize {
        let (tx, ty) = (tx as usize, ty as usize);
        let step = if self.bpp8 { 2 } else { 1 };
        let idx = if self.map_1d {
            self.tile + ((ty >> 3) * (self.w as usize >> 3) + (tx >> 3)) * step
        } else {
            // 2D: 32 tiles (1 KiB) per tile row; in 8bpp the low bit of the base is ignored.
            let base = if self.bpp8 { self.tile & !1 } else { self.tile };
            base + (ty >> 3) * 32 + (tx >> 3) * step
        } & 0x3FF;
        // Bitmap modes: the lower 16 KiB of OBJ VRAM belongs to the bitmap; tiles 0-511 are not displayed.
        if self.bitmap_mode && idx < 512 {
            return 0;
        }
        let off = if self.bpp8 {
            idx * 32 + (ty & 7) * 8 + (tx & 7)
        } else {
            idx * 32 + (ty & 7) * 4 + ((tx & 7) >> 1)
        };
        let b = mem.vram[0x1_0000 + (off & 0x7FFF)] as usize;
        if self.bpp8 {
            b
        } else if tx & 1 != 0 {
            b >> 4
        } else {
            b & 0xF
        }
    }
}

pub fn render(mem: &Mem, y: usize, out: &mut ObjLine) {
    out.clear();
    let dispcnt = mem.dispcnt();
    if dispcnt & (1 << 12) == 0 {
        return;
    }
    let map_1d = dispcnt & (1 << 6) != 0;
    let bitmap_mode = dispcnt & 7 >= 3;
    let objwin_enabled = dispcnt & (1 << 15) != 0;
    let mosaic = mem.mosaic();

    for i in 0..128 {
        let a0 = mem.oam16(i * 8);
        let a1 = mem.oam16(i * 8 + 2);
        let a2 = mem.oam16(i * 8 + 4);

        let affine = a0 & (1 << 8) != 0;
        if !affine && a0 & (1 << 9) != 0 {
            continue; // OBJ disable
        }
        let mode = (a0 >> 10) & 3;
        if mode == 2 && !objwin_enabled {
            continue;
        }
        let shape = (a0 >> 14) as usize;
        if shape == 3 {
            continue; // prohibited
        }
        let size = (a1 >> 14) as usize;
        let (w, h) = SIZES[shape][size];
        let double = affine && a0 & (1 << 9) != 0;
        let (rw, rh) = if double { (w * 2, h * 2) } else { (w, h) };

        let obj_y = (a0 & 0xFF) as i32;
        let mut obj_x = (a1 & 0x1FF) as i32;
        if obj_x >= 256 {
            obj_x -= 512;
        }
        if obj_x >= SCREEN_W as i32 || obj_x + rw <= 0 {
            continue;
        }

        let use_mosaic = a0 & (1 << 12) != 0;
        let mut ly = y as i32;
        if use_mosaic {
            ly -= ly % mosaic.obj_v as i32;
        }
        let y_local = (ly - obj_y) & 0xFF;
        if y_local >= rh {
            continue;
        }

        let sprite = Sprite {
            w,
            tile: (a2 & 0x3FF) as usize,
            bpp8: a0 & (1 << 13) != 0,
            pal_bank: ((a2 >> 12) & 0xF) as usize * 16,
            map_1d,
            bitmap_mode,
        };
        let prio = ((a2 >> 10) & 3) as u8;

        let (pa, pb, pc, pd) = if affine {
            let g = ((a1 >> 9) & 0x1F) as usize * 32;
            (
                mem.oam16(g + 0x06) as i16 as i32,
                mem.oam16(g + 0x0E) as i16 as i32,
                mem.oam16(g + 0x16) as i16 as i32,
                mem.oam16(g + 0x1E) as i16 as i32,
            )
        } else {
            (0x100, 0, 0, 0x100)
        };
        let hflip = !affine && a1 & (1 << 12) != 0;
        let vflip = !affine && a1 & (1 << 13) != 0;

        let x_start = obj_x.max(0);
        let x_end = (obj_x + rw).min(SCREEN_W as i32);
        let iy = y_local - rh / 2;
        for x in x_start..x_end {
            let sx_screen = if use_mosaic { x - x % mosaic.obj_h as i32 } else { x };
            let sx = sx_screen - obj_x;
            if sx < 0 {
                continue;
            }
            let (tx, ty) = if affine {
                let ix = sx - rw / 2;
                let tx = ((pa * ix + pb * iy) >> 8) + w / 2;
                let ty = ((pc * ix + pd * iy) >> 8) + h / 2;
                if tx < 0 || ty < 0 || tx >= w || ty >= h {
                    continue;
                }
                (tx, ty)
            } else {
                (if hflip { w - 1 - sx } else { sx }, if vflip { h - 1 - y_local } else { y_local })
            };
            let idx = sprite.texel(mem, tx, ty);
            if idx == 0 {
                continue;
            }
            let x = x as usize;
            if mode == 2 {
                out.window[x] = true;
                continue;
            }
            if out.color[x] == TRANSPARENT || prio < out.prio[x] {
                out.color[x] = mem.pal_color(256 + if sprite.bpp8 { idx } else { sprite.pal_bank + idx });
                out.prio[x] = prio;
                out.semi[x] = mode == 1;
            }
        }
    }
}
