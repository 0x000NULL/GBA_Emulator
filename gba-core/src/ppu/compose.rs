//! Per-pixel composition: window masks, layer priority, colour special effects, mosaic helpers
//! and green swap.
//! Source: GBATEK "LCD I/O Window Feature", "Color Special Effects", "BG Control" (priority),
//! "OAM Attributes" (OBJ priority relative to BG), "Display Control" (green swap).

use super::obj::ObjLine;
use super::regs::{Blend, Mem, Window, LAYER_BACKDROP, LAYER_OBJ, TRANSPARENT};
use crate::SCREEN_W;

/// Fill `out` with the per-pixel window control byte (bits 0-3 BG0-3, 4 OBJ, 5 effects).
/// When no window is enabled everything is on. Window 0 has the highest priority, then window 1,
/// then the OBJ window, then the outside region.
pub fn window_line(mem: &Mem, y: usize, obj_win: &[bool; SCREEN_W], out: &mut [u8; SCREEN_W]) {
    let dispcnt = mem.dispcnt();
    if dispcnt & 0xE000 == 0 {
        out.fill(0x3F);
        return;
    }
    out.fill(mem.win_out());
    if dispcnt & (1 << 15) != 0 {
        let ctrl = mem.win_obj();
        for (o, &w) in out.iter_mut().zip(obj_win.iter()) {
            if w {
                *o = ctrl;
            }
        }
    }
    if dispcnt & (1 << 14) != 0 {
        apply_window(&mem.window(1), y as u32, mem.win_in(1), out);
    }
    if dispcnt & (1 << 13) != 0 {
        apply_window(&mem.window(0), y as u32, mem.win_in(0), out);
    }
}

/// Window range semantics (matching hardware counter-compare behaviour as mGBA models it):
/// vertically, `y1 <= y2` means `y1 <= y < y2`, otherwise the range wraps (`y >= y1 || y < y2`).
/// Horizontally the same, with `x2 > 240` clamped to 240 (GBATEK: "garbage values of X2>240 or
/// X1>X2 are interpreted as X2=240"); an `x1 > 240` beyond that clamp is clamped too, so a
/// window such as WIN0H=FEFFh (X1=254, X2=255) is empty rather than wrapping over the whole
/// line (Super Mario Advance 2 writes exactly that every H-Blank to hide a window and relies on
/// the level staying visible). An unreachable `x1 > 240` with `x1 > x2` is treated as 0 (mGBA).
fn apply_window(win: &Window, y: u32, ctrl: u8, out: &mut [u8; SCREEN_W]) {
    let in_y = if win.y1 <= win.y2 { y >= win.y1 && y < win.y2 } else { y >= win.y1 || y < win.y2 };
    if !in_y {
        return;
    }
    let mut x1 = win.x1 as usize;
    let mut x2 = win.x2 as usize;
    if x1 > SCREEN_W && x1 > x2 {
        x1 = 0;
    }
    if x2 > SCREEN_W {
        x2 = SCREEN_W;
        if x1 > SCREEN_W {
            x1 = SCREEN_W;
        }
    }
    if x2 < x1 {
        out[..x2].fill(ctrl);
        out[x1.min(SCREEN_W)..].fill(ctrl);
    } else {
        out[x1.min(SCREEN_W)..x2].fill(ctrl);
    }
}

/// Horizontal mosaic over a finished layer line: pixel x takes the colour of pixel x - x % h.
pub fn mosaic_h(line: &mut [u16; SCREEN_W], h: usize) {
    if h <= 1 {
        return;
    }
    for x in 0..SCREEN_W {
        let src = x - x % h;
        line[x] = line[src];
    }
}

#[inline]
fn alpha_blend(c1: u16, c2: u16, eva: u32, evb: u32) -> u16 {
    let mut out = 0u16;
    for shift in [0u32, 5, 10] {
        let a = ((c1 >> shift) & 0x1F) as u32;
        let b = ((c2 >> shift) & 0x1F) as u32;
        let v = ((a * eva + b * evb) >> 4).min(31);
        out |= (v as u16) << shift;
    }
    out
}

#[inline]
fn brightness(c: u16, evy: u32, increase: bool) -> u16 {
    let mut out = 0u16;
    for shift in [0u32, 5, 10] {
        let i = ((c >> shift) & 0x1F) as u32;
        let v = if increase { i + (((31 - i) * evy) >> 4) } else { i - ((i * evy) >> 4) };
        out |= (v as u16) << shift;
    }
    out
}

/// Compose one line into `row`. `bg_line[i]` is only consulted when `bg_active[i]`.
pub fn compose_line(
    mem: &Mem,
    bg_line: &[[u16; SCREEN_W]; 4],
    bg_active: [bool; 4],
    obj: &ObjLine,
    win: &[u8; SCREEN_W],
    row: &mut [u16],
) {
    // BG draw order: by priority, then by index (BG0 highest at equal priority).
    let mut order: [(u8, u8); 4] = [(0, 0); 4];
    let mut n = 0;
    for (bg, &active) in bg_active.iter().enumerate() {
        if active {
            order[n] = (mem.bgcnt(bg).prio, bg as u8);
            n += 1;
        }
    }
    order[..n].sort_unstable();

    let blend: Blend = mem.blend();
    let backdrop = mem.pal_color(0);

    for (x, out) in row.iter_mut().enumerate() {
        let mask = win[x];
        let obj_ok = mask & 0x10 != 0 && obj.color[x] != TRANSPARENT;
        let obj_prio = obj.prio[x];

        // Top two visible layers, in order.
        let mut layers: [(u8, u16); 2] = [(LAYER_BACKDROP, backdrop); 2];
        let mut count = 0usize;
        let mut bi = 0usize;
        'search: for p in 0..4u8 {
            if obj_ok && obj_prio == p {
                layers[count] = (LAYER_OBJ, obj.color[x]);
                count += 1;
                if count == 2 {
                    break 'search;
                }
            }
            while bi < n && order[bi].0 == p {
                let bg = order[bi].1 as usize;
                bi += 1;
                let c = bg_line[bg][x];
                if mask & (1 << bg) != 0 && c != TRANSPARENT {
                    layers[count] = (bg as u8, c);
                    count += 1;
                    if count == 2 {
                        break 'search;
                    }
                }
            }
        }
        if count < 2 {
            layers[count] = (LAYER_BACKDROP, backdrop);
            count += 1;
        }

        let (l1, c1) = layers[0];
        let mut color = c1;
        if mask & 0x20 != 0 {
            let second = if count == 2 { Some(layers[1]) } else { None };
            let second_is_t2 = second.is_some_and(|(l2, _)| blend.target2 & (1 << l2) != 0);
            let semi = l1 == LAYER_OBJ && obj.semi[x];
            if semi && second_is_t2 {
                color = alpha_blend(c1, second.unwrap().1, blend.eva, blend.evb);
            } else if blend.target1 & (1 << l1) != 0 {
                match blend.mode {
                    1 if second_is_t2 => color = alpha_blend(c1, second.unwrap().1, blend.eva, blend.evb),
                    2 => color = brightness(c1, blend.evy, true),
                    3 => color = brightness(c1, blend.evy, false),
                    _ => {}
                }
            }
        }
        *out = color;
    }
}

/// Green swap (DISPCNT+2 bit 0): exchange the green component of each even/odd pixel pair.
pub fn green_swap(row: &mut [u16]) {
    for pair in row.chunks_exact_mut(2) {
        let g0 = pair[0] & 0x03E0;
        let g1 = pair[1] & 0x03E0;
        pair[0] = (pair[0] & !0x03E0) | g1;
        pair[1] = (pair[1] & !0x03E0) | g0;
    }
}
