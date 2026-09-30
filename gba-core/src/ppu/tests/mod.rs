//! PPU rendering unit tests: fill PAL/VRAM/OAM/IO by hand, render a line (`prepare_obj` +
//! `render_scanline`, as the H-Blank handlers do), assert pixels.

use super::*;

const W: usize = SCREEN_W;

// ---------------------------------------------------------------- helpers

fn ppu() -> Ppu {
    Ppu::new()
}

fn set16(p: &mut Ppu, off: usize, v: u16) {
    p.io[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn set32(p: &mut Ppu, off: usize, v: u32) {
    p.io[off..off + 4].copy_from_slice(&v.to_le_bytes());
    // Mirror the bus behaviour: writes to reference points reload the internal copy.
    p.latch_affine_refs();
}

fn pal(p: &mut Ppu, idx: usize, color: u16) {
    p.pal[idx * 2..idx * 2 + 2].copy_from_slice(&color.to_le_bytes());
}

fn vram16(p: &mut Ppu, addr: usize, v: u16) {
    p.vram[addr..addr + 2].copy_from_slice(&v.to_le_bytes());
}

fn oam16(p: &mut Ppu, addr: usize, v: u16) {
    p.oam[addr..addr + 2].copy_from_slice(&v.to_le_bytes());
}

/// Solid 4bpp tile (32 bytes) with every pixel = `idx`.
fn tile4_solid(p: &mut Ppu, addr: usize, idx: u8) {
    p.vram[addr..addr + 32].fill(idx | (idx << 4));
}

/// 4bpp tile whose pixel (x, y) = f(x, y).
fn tile4(p: &mut Ppu, addr: usize, f: impl Fn(usize, usize) -> u8) {
    for y in 0..8 {
        for x in 0..8 {
            let v = f(x, y) & 0xF;
            let b = &mut p.vram[addr + y * 4 + x / 2];
            if x & 1 == 0 {
                *b = (*b & 0xF0) | v;
            } else {
                *b = (*b & 0x0F) | (v << 4);
            }
        }
    }
}

/// 8bpp tile whose pixel (x, y) = f(x, y).
fn tile8(p: &mut Ppu, addr: usize, f: impl Fn(usize, usize) -> u8) {
    for y in 0..8 {
        for x in 0..8 {
            p.vram[addr + y * 8 + x] = f(x, y);
        }
    }
}

/// Render lines 0..=y from a fresh frame start, so the internal affine reference points hold
/// the value they would have on line `y` of a real frame.
fn frame_line(p: &mut Ppu, y: usize) {
    p.latch_affine_refs();
    for i in 0..=y {
        render(p, i);
    }
}

/// Evaluate OAM for line `y` and render it, as consecutive H-Blank handlers would.
fn render(p: &mut Ppu, y: usize) {
    p.prepare_obj(y);
    p.render_scanline(y);
}

fn row(p: &Ppu, y: usize) -> &[u16] {
    &p.fb[y * W..(y + 1) * W]
}

fn px(p: &Ppu, x: usize, y: usize) -> u16 {
    p.fb[y * W + x]
}

const RED: u16 = 0x001F;
const GREEN: u16 = 0x03E0;
const BLUE: u16 = 0x7C00;
const WHITE: u16 = 0x7FFF;
const BACKDROP: u16 = 0x1234;

/// Mode 0, BG0 on, backdrop = BACKDROP, palette 1..3 = red/green/blue.
fn basic_mode0(p: &mut Ppu) {
    set16(p, 0x00, 1 << 8);
    pal(p, 0, BACKDROP);
    pal(p, 1, RED);
    pal(p, 2, GREEN);
    pal(p, 3, BLUE);
}

/// BGxCNT value: prio, char base block, screen base block, size, bpp8.
fn bgcnt(prio: u16, char_block: u16, screen_block: u16, size: u16, bpp8: bool) -> u16 {
    prio | char_block << 2 | (bpp8 as u16) << 7 | screen_block << 8 | size << 14
}

// ---------------------------------------------------------------- forced blank / backdrop

#[test]
fn forced_blank_is_white() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x00, 0x80 | 1 << 8);
    render(&mut p, 10);
    assert!(row(&p, 10).iter().all(|&c| c == WHITE));
}

#[test]
fn empty_screen_is_backdrop() {
    let mut p = ppu();
    basic_mode0(&mut p);
    render(&mut p, 0);
    assert!(row(&p, 0).iter().all(|&c| c == BACKDROP));
}

#[test]
fn backdrop_ignores_bit15() {
    let mut p = ppu();
    pal(&mut p, 0, 0x8000 | BACKDROP);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
}

// ---------------------------------------------------------------- text BG

#[test]
fn mode0_text_4bpp_single_tile() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false)); // tiles at 0, map at 0x4000
    tile4_solid(&mut p, 32, 1); // tile 1 = red
    vram16(&mut p, 0x4000 + 2, 1); // map (1,0) = tile 1
    render(&mut p, 3);
    assert_eq!(px(&p, 7, 3), BACKDROP);
    for x in 8..16 {
        assert_eq!(px(&p, x, 3), RED);
    }
    assert_eq!(px(&p, 16, 3), BACKDROP);
    render(&mut p, 8);
    assert_eq!(px(&p, 8, 8), BACKDROP, "tile row 1 is empty");
}

#[test]
fn text_4bpp_palette_bank_and_transparency() {
    let mut p = ppu();
    basic_mode0(&mut p);
    pal(&mut p, 16 + 1, BLUE);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4(&mut p, 32, |x, _| if x < 4 { 1 } else { 0 });
    vram16(&mut p, 0x4000, 1 | 1 << 12); // palette bank 1
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BLUE);
    assert_eq!(px(&p, 3, 0), BLUE);
    assert_eq!(px(&p, 4, 0), BACKDROP, "colour 0 is transparent");
}

#[test]
fn text_8bpp() {
    let mut p = ppu();
    basic_mode0(&mut p);
    pal(&mut p, 200, GREEN);
    set16(&mut p, 0x08, bgcnt(0, 1, 0, 0, true)); // tiles at 0x4000, map at 0
    tile8(&mut p, 0x4000 + 64 * 2, |x, y| if x == y { 200 } else { 0 });
    vram16(&mut p, 0, 2);
    for y in 0..8 {
        render(&mut p, y);
        for x in 0..8 {
            assert_eq!(px(&p, x, y), if x == y { GREEN } else { BACKDROP }, "({x},{y})");
        }
    }
}

#[test]
fn text_flips() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    // Tile 1: pixel (0,0) = 1 (red), (7,7) = 2 (green), rest 0.
    tile4(&mut p, 32, |x, y| match (x, y) {
        (0, 0) => 1,
        (7, 7) => 2,
        _ => 0,
    });
    vram16(&mut p, 0x4000, 1); // normal at x 0..8
    vram16(&mut p, 0x4002, 1 | 1 << 10); // hflip at 8..16
    vram16(&mut p, 0x4004, 1 | 1 << 11); // vflip at 16..24
    vram16(&mut p, 0x4006, 1 | 3 << 10); // both at 24..32
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 15, 0), RED);
    assert_eq!(px(&p, 23, 0), GREEN);
    assert_eq!(px(&p, 24, 0), GREEN);
    render(&mut p, 7);
    assert_eq!(px(&p, 7, 7), GREEN);
    assert_eq!(px(&p, 8, 7), GREEN);
    assert_eq!(px(&p, 16, 7), RED);
    assert_eq!(px(&p, 31, 7), RED);
}

#[test]
fn text_scroll_and_wrap_256() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4_solid(&mut p, 32, 1);
    vram16(&mut p, 0x4000, 1); // tile (0,0) red
    set16(&mut p, 0x10, 0x1F8 + 4); // hofs = 508 -> effective 252: tile 0 appears at x 4..12
    set16(&mut p, 0x12, 0x100 + 3); // vofs = 259 -> effective 3: tile row 0 at y 0..5
    render(&mut p, 0);
    assert_eq!(px(&p, 3, 0), BACKDROP);
    assert_eq!(px(&p, 4, 0), RED);
    assert_eq!(px(&p, 11, 0), RED);
    assert_eq!(px(&p, 12, 0), BACKDROP);
    render(&mut p, 5);
    assert_eq!(px(&p, 4, 5), BACKDROP);
}

#[test]
fn text_512_wide_uses_second_screen_block() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 1, false)); // 512x256, SC0 at 0x4000, SC1 at 0x4800
    tile4_solid(&mut p, 32, 1);
    tile4_solid(&mut p, 64, 2);
    vram16(&mut p, 0x4000, 1); // SC0 (0,0) red
    vram16(&mut p, 0x4800, 2); // SC1 (0,0) = world x 256 green
    set16(&mut p, 0x10, 248); // world x 248..488 visible: red at x<8? no: world 248..255 is SC0 tile 31 (empty), 256..264 SC1 tile 0
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
    assert_eq!(px(&p, 8, 0), GREEN);
    assert_eq!(px(&p, 15, 0), GREEN);
    assert_eq!(px(&p, 16, 0), BACKDROP);
    set16(&mut p, 0x10, 504); // world 504..511 then wraps to 0..: red at x 8..16
    render(&mut p, 1);
    assert_eq!(px(&p, 8, 1), RED);
}

#[test]
fn text_256x512_and_512x512_screen_blocks() {
    let mut p = ppu();
    basic_mode0(&mut p);
    tile4_solid(&mut p, 32, 1);
    tile4_solid(&mut p, 64, 2);
    tile4_solid(&mut p, 96, 3);
    // Size 2 (256x512): SC1 at +0x800 holds y 256..511.
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 2, false));
    vram16(&mut p, 0x4800, 1);
    set16(&mut p, 0x12, 256);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    // Size 3 (512x512): SC2 at +0x1000 holds (x<256, y>=256), SC3 at +0x1800 holds (x>=256, y>=256).
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 3, false));
    vram16(&mut p, 0x5000, 2);
    vram16(&mut p, 0x5800, 3);
    set16(&mut p, 0x10, 0);
    render(&mut p, 1);
    assert_eq!(px(&p, 0, 1), GREEN);
    set16(&mut p, 0x10, 256);
    render(&mut p, 2);
    assert_eq!(px(&p, 0, 2), BLUE);
}

#[test]
fn text_tiles_in_obj_vram_are_transparent() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x08, bgcnt(0, 3, 0, 0, false)); // char base 0xC000
    p.vram[0x10000..0x10020].fill(0x11);
    vram16(&mut p, 0, 0x200); // tile 512 at 0xC000 + 0x4000 = 0x10000
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
}

// ---------------------------------------------------------------- priority

#[test]
fn bg_priority_lower_number_wins_then_lower_index() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x00, 0xF << 8);
    tile4_solid(&mut p, 32, 1); // red
    tile4_solid(&mut p, 64, 2); // green
    tile4_solid(&mut p, 96, 3); // blue
    // BG0 prio 1 red, BG1 prio 0 green, BG2 prio 0 blue (BG1 wins over BG2 at equal prio).
    set16(&mut p, 0x08, bgcnt(1, 0, 8, 0, false));
    set16(&mut p, 0x0A, bgcnt(0, 0, 9, 0, false));
    set16(&mut p, 0x0C, bgcnt(0, 0, 10, 0, false));
    vram16(&mut p, 0x4000, 1);
    vram16(&mut p, 0x4800, 2);
    vram16(&mut p, 0x5000, 3);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
    // Make BG1 transparent there: BG2 shows.
    vram16(&mut p, 0x4800, 0);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BLUE);
    vram16(&mut p, 0x5000, 0);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
}

#[test]
fn dispcnt_disables_layers_and_mode1_bg3_absent() {
    let mut p = ppu();
    basic_mode0(&mut p);
    tile4_solid(&mut p, 32, 1);
    set16(&mut p, 0x0E, bgcnt(0, 0, 8, 0, false));
    vram16(&mut p, 0x4000, 1);
    set16(&mut p, 0x00, 1 << 11); // mode 0, BG3 on
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    set16(&mut p, 0x00, 1 | 1 << 11); // mode 1: BG3 does not exist
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
    set16(&mut p, 0x00, 0); // BG3 off
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
}

// ---------------------------------------------------------------- bitmap modes

#[test]
fn mode3_raw_colours() {
    let mut p = ppu();
    set16(&mut p, 0x00, 3 | 1 << 10);
    set16(&mut p, 0x20, 0x100);
    set16(&mut p, 0x26, 0x100);
    vram16(&mut p, (5 * 240 + 17) * 2, 0x8000 | GREEN);
    frame_line(&mut p, 5);
    assert_eq!(px(&p, 17, 5), GREEN);
    assert_eq!(px(&p, 18, 5), 0);
}

#[test]
fn mode4_frame_select_and_transparency() {
    let mut p = ppu();
    pal(&mut p, 0, BACKDROP);
    pal(&mut p, 7, RED);
    pal(&mut p, 9, BLUE);
    set16(&mut p, 0x00, 4 | 1 << 10);
    set16(&mut p, 0x20, 0x100);
    set16(&mut p, 0x26, 0x100);
    p.vram[20 * 240 + 100] = 7;
    p.vram[0xA000 + 20 * 240 + 100] = 9;
    frame_line(&mut p, 20);
    assert_eq!(px(&p, 100, 20), RED);
    assert_eq!(px(&p, 101, 20), BACKDROP);
    set16(&mut p, 0x00, 4 | 1 << 4 | 1 << 10);
    frame_line(&mut p, 20);
    assert_eq!(px(&p, 100, 20), BLUE);
}

#[test]
fn mode5_bounds_and_frame() {
    let mut p = ppu();
    pal(&mut p, 0, BACKDROP);
    set16(&mut p, 0x00, 5 | 1 << 10);
    set16(&mut p, 0x20, 0x100);
    set16(&mut p, 0x26, 0x100);
    for i in 0..160 * 128 {
        vram16(&mut p, i * 2, RED);
        vram16(&mut p, 0xA000 + i * 2, GREEN);
    }
    frame_line(&mut p, 128);
    assert_eq!(px(&p, 159, 127), RED);
    assert_eq!(px(&p, 160, 127), BACKDROP);
    assert_eq!(px(&p, 0, 128), BACKDROP);
    set16(&mut p, 0x00, 5 | 1 << 4 | 1 << 10);
    frame_line(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
}

#[test]
fn mode3_uses_affine_reference_and_never_wraps() {
    let mut p = ppu();
    pal(&mut p, 0, BACKDROP);
    set16(&mut p, 0x00, 3 | 1 << 10);
    set16(&mut p, 0x0C, 1 << 13); // wrap bit must be ignored in bitmap modes
    set16(&mut p, 0x20, 0x100);
    set16(&mut p, 0x26, 0x100);
    for i in 0..240 * 160 {
        vram16(&mut p, i * 2, RED);
    }
    set32(&mut p, 0x28, (-10i32 << 8) as u32); // BG2X = -10
    render(&mut p, 0);
    assert_eq!(px(&p, 9, 0), BACKDROP);
    assert_eq!(px(&p, 10, 0), RED);
}

// ---------------------------------------------------------------- affine BG

/// Mode 2, BG2 128x128 8bpp: map at 0x4000, tiles at 0. Tile 1 is a solid `1` (red), tile 2 has
/// only its top-left pixel set (green). Map places tile 1 at (0,0) and tile 2 at tile (1,0).
fn affine_setup(p: &mut Ppu, wrap: bool) {
    pal(p, 0, BACKDROP);
    pal(p, 1, RED);
    pal(p, 2, GREEN);
    set16(p, 0x00, 2 | 1 << 10);
    set16(p, 0x0C, bgcnt(0, 0, 8, 0, true) | (wrap as u16) << 13);
    tile8(p, 64, |_, _| 1);
    tile8(p, 128, |x, y| if x == 0 && y == 0 { 2 } else { 0 });
    p.vram[0x4000] = 1;
    p.vram[0x4001] = 2;
    set16(p, 0x20, 0x100);
    set16(p, 0x22, 0);
    set16(p, 0x24, 0);
    set16(p, 0x26, 0x100);
    set32(p, 0x28, 0);
    set32(p, 0x2C, 0);
}

#[test]
fn affine_identity() {
    let mut p = ppu();
    affine_setup(&mut p, false);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 7, 0), RED);
    assert_eq!(px(&p, 8, 0), GREEN);
    assert_eq!(px(&p, 9, 0), BACKDROP);
    assert_eq!(px(&p, 127, 0), BACKDROP);
    assert_eq!(px(&p, 128, 0), BACKDROP, "outside 128x128 is transparent without wrap");
}

#[test]
fn affine_wraparound() {
    let mut p = ppu();
    affine_setup(&mut p, true);
    render(&mut p, 0);
    assert_eq!(px(&p, 128, 0), RED, "x 128 wraps to 0");
    assert_eq!(px(&p, 136, 0), GREEN);
}

#[test]
fn affine_scale_2x() {
    let mut p = ppu();
    affine_setup(&mut p, false);
    set16(&mut p, 0x20, 0x80); // PA = 0.5 -> texture advances half a pixel per screen pixel
    set16(&mut p, 0x26, 0x80);
    render(&mut p, 0);
    assert_eq!(px(&p, 15, 0), RED);
    assert_eq!(px(&p, 16, 0), GREEN);
    assert_eq!(px(&p, 17, 0), GREEN);
    assert_eq!(px(&p, 18, 0), BACKDROP);
    render(&mut p, 1);
    assert_eq!(px(&p, 16, 1), GREEN, "texture row 0 covers screen rows 0 and 1");
    render(&mut p, 2);
    assert_eq!(px(&p, 16, 2), BACKDROP);
}

#[test]
fn affine_rotate_90() {
    // PA=0, PB=-1, PC=1, PD=0: x2 = -(y1) , y2 = x1 ; with ref (8, 0):  screen (x,y) -> tex (8 - y, x).
    // Texture pixel (8,0) (green) lands at screen (0,0); red tile column 0..8 lands at y 1..8 of x 0..8.
    let mut p = ppu();
    affine_setup(&mut p, false);
    set16(&mut p, 0x20, 0);
    set16(&mut p, 0x22, (-0x100i16) as u16);
    set16(&mut p, 0x24, 0x100);
    set16(&mut p, 0x26, 0);
    set32(&mut p, 0x28, 8 << 8);
    set32(&mut p, 0x2C, 0);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
    assert_eq!(px(&p, 1, 0), BACKDROP, "tex (8,1) is transparent");
    // The reference point advances by PB/PD per line: line 1 -> ref (7, 0) -> tex (7 - 0, x) = red column.
    render(&mut p, 1);
    assert_eq!(px(&p, 0, 1), RED);
    assert_eq!(px(&p, 7, 1), RED);
    assert_eq!(px(&p, 8, 1), BACKDROP, "tex y 8 is tile row 1 (empty)");
}

#[test]
fn affine_reference_advances_and_reloads() {
    let mut p = ppu();
    affine_setup(&mut p, false);
    set16(&mut p, 0x26, 0x100); // PD = 1
    // Lines 0..7 show tile row 0 (red); after 8 lines the reference has moved 8 texels down.
    for y in 0..8 {
        render(&mut p, y);
        assert_eq!(px(&p, 0, y), RED);
    }
    render(&mut p, 8);
    assert_eq!(px(&p, 0, 8), BACKDROP);
    // Writing BG2Y mid-frame reloads the internal register for the next line.
    set32(&mut p, 0x2C, 0);
    render(&mut p, 9);
    assert_eq!(px(&p, 0, 9), RED);
    // Frame start (latch at line 0) resets it too.
    for y in 10..20 {
        render(&mut p, y);
    }
    p.latch_affine_refs();
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
}

#[test]
fn affine_reference_is_28bit_signed() {
    let mut p = ppu();
    affine_setup(&mut p, false);
    set32(&mut p, 0x28, 0x0FFF_FF00); // -1.0 in 28-bit
    assert_eq!(p.bg_ref[0], -256);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
    assert_eq!(px(&p, 1, 0), RED);
}

#[test]
fn affine_sizes() {
    let mut p = ppu();
    affine_setup(&mut p, false);
    // Size 3 = 1024x1024, 128 tiles per map row. Put tile 1 at tile (0, 1) -> texel y 8..16.
    set16(&mut p, 0x0C, bgcnt(0, 0, 8, 3, true));
    p.vram[0x4000] = 0;
    p.vram[0x4001] = 0;
    p.vram[0x4000 + 128] = 1;
    frame_line(&mut p, 8);
    assert_eq!(px(&p, 0, 7), BACKDROP);
    assert_eq!(px(&p, 0, 8), RED);
    assert_eq!(px(&p, 8, 8), BACKDROP);
}

// ---------------------------------------------------------------- OBJ

/// Mode 0 with OBJ on, OBJ palette 1 = red, 2 = green, 3 = blue; OBJ tile 1 solid `1`.
fn obj_setup(p: &mut Ppu) {
    pal(p, 0, BACKDROP);
    pal(p, 256 + 1, RED);
    pal(p, 256 + 2, GREEN);
    pal(p, 256 + 3, BLUE);
    pal(p, 256 + 16 + 1, BLUE); // bank 1 colour 1
    set16(p, 0x00, 1 << 12);
    p.vram[0x10020..0x10040].fill(0x11); // tile 1 = solid 1
    p.vram[0x10040..0x10060].fill(0x22); // tile 2 = solid 2
}

fn obj(p: &mut Ppu, i: usize, a0: u16, a1: u16, a2: u16) {
    oam16(p, i * 8, a0);
    oam16(p, i * 8 + 2, a1);
    oam16(p, i * 8 + 4, a2);
}

fn hide_all_objs(p: &mut Ppu) {
    for i in 0..128 {
        oam16(p, i * 8, 1 << 9);
    }
}

#[test]
fn obj_regular_8x8_position() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    obj(&mut p, 0, 20, 30, 1); // y 20, x 30, tile 1
    render(&mut p, 19);
    assert_eq!(px(&p, 30, 19), BACKDROP);
    render(&mut p, 20);
    assert_eq!(px(&p, 29, 20), BACKDROP);
    assert_eq!(px(&p, 30, 20), RED);
    assert_eq!(px(&p, 37, 20), RED);
    assert_eq!(px(&p, 38, 20), BACKDROP);
    render(&mut p, 28);
    assert_eq!(px(&p, 30, 28), BACKDROP);
}

#[test]
fn obj_hidden_and_master_disable() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    obj(&mut p, 0, 0 | 1 << 9, 0, 1);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
    obj(&mut p, 0, 0, 0, 1);
    set16(&mut p, 0x00, 0);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
}

#[test]
fn obj_x_negative_and_y_wrap() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    obj(&mut p, 0, 252, 508, 1); // x = -4, y = 252 -> rows 252..255 offscreen, 0..3 visible at top
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 3, 0), RED);
    assert_eq!(px(&p, 4, 0), BACKDROP);
    render(&mut p, 4);
    assert_eq!(px(&p, 0, 4), BACKDROP);
}

#[test]
fn obj_shapes_and_sizes() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    set16(&mut p, 0x00, 1 << 12 | 1 << 6); // 1D mapping
    p.vram[0x10020..0x10000 + 0x20 * 65].fill(0x11); // tiles 1..=64 solid
    // Horizontal 64x32 at (0,0).
    obj(&mut p, 0, 1 << 14, 3 << 14, 1);
    render(&mut p, 31);
    assert_eq!(px(&p, 63, 31), RED);
    assert_eq!(px(&p, 64, 31), BACKDROP);
    render(&mut p, 32);
    assert_eq!(px(&p, 0, 32), BACKDROP);
    // Vertical 8x32 at (100, 0).
    obj(&mut p, 0, 2 << 14, 1 << 14 | 100, 1);
    render(&mut p, 31);
    assert_eq!(px(&p, 100, 31), RED);
    assert_eq!(px(&p, 108, 31), BACKDROP);
    // Square 64x64.
    obj(&mut p, 0, 0, 3 << 14, 1);
    render(&mut p, 63);
    assert_eq!(px(&p, 63, 63), RED);
    render(&mut p, 64);
    assert_eq!(px(&p, 0, 64), BACKDROP);
}

#[test]
fn obj_mapping_1d_vs_2d() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    // Tiles: 4 = solid 1 (red), 5 = solid 2 (green), 6 = solid 3 (blue), 0x24 = solid 3.
    p.vram[0x10000 + 4 * 32..0x10000 + 5 * 32].fill(0x11);
    p.vram[0x10000 + 5 * 32..0x10000 + 6 * 32].fill(0x22);
    p.vram[0x10000 + 6 * 32..0x10000 + 7 * 32].fill(0x33);
    p.vram[0x10000 + 0x24 * 32..0x10000 + 0x25 * 32].fill(0x33);
    p.vram[0x10000 + 0x25 * 32..0x10000 + 0x26 * 32].fill(0x11);
    obj(&mut p, 0, 0, 1 << 14, 4); // 16x16 at (0,0), tile 4
    // 2D: rows are tiles 4,5 / 0x24,0x25.
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 8, 0), GREEN);
    render(&mut p, 8);
    assert_eq!(px(&p, 0, 8), BLUE);
    assert_eq!(px(&p, 8, 8), RED);
    // 1D: rows are tiles 4,5 / 6,7 (7 empty).
    set16(&mut p, 0x00, 1 << 12 | 1 << 6);
    render(&mut p, 8);
    assert_eq!(px(&p, 0, 8), BLUE);
    assert_eq!(px(&p, 8, 8), BACKDROP);
}

#[test]
fn obj_8bpp_mapping() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    pal(&mut p, 256 + 77, GREEN);
    pal(&mut p, 256 + 78, BLUE);
    // 8bpp tile "4" occupies 64 bytes at 4*32; tile "6" at 6*32.
    p.vram[0x10000 + 4 * 32..0x10000 + 6 * 32].fill(77);
    p.vram[0x10000 + 6 * 32..0x10000 + 8 * 32].fill(78);
    // 2D mapping, tile 0x24 (row below in 2D 8bpp = +32) filled with 78.
    p.vram[0x10000 + 0x24 * 32..0x10000 + 0x26 * 32].fill(78);
    obj(&mut p, 0, 1 << 13, 1 << 14, 4); // 8bpp 16x16
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
    assert_eq!(px(&p, 8, 0), BLUE); // 2D: next tile column is +2 = tile 6
    render(&mut p, 8);
    assert_eq!(px(&p, 0, 8), BLUE); // 2D: next row is +32 = 0x24
    // 1D 8bpp: row 1 starts at 4 + 2*2 = 8 (empty).
    set16(&mut p, 0x00, 1 << 12 | 1 << 6);
    render(&mut p, 8);
    assert_eq!(px(&p, 0, 8), BACKDROP);
    // 2D 8bpp ignores the low bit of the tile number.
    set16(&mut p, 0x00, 1 << 12);
    obj(&mut p, 0, 1 << 13, 1 << 14, 5);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
}

#[test]
fn obj_flips() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    // Tile 3: only pixel (0,0) = 1.
    p.vram[0x10060..0x10080].fill(0);
    p.vram[0x10060] = 0x01;
    obj(&mut p, 0, 0, 0, 3);
    obj(&mut p, 1, 0, 1 << 12 | 8, 3); // hflip at x 8
    obj(&mut p, 2, 0, 1 << 13 | 16, 3); // vflip at x 16
    obj(&mut p, 3, 0, 3 << 12 | 24, 3); // both at x 24
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 15, 0), RED);
    assert_eq!(px(&p, 16, 0), BACKDROP);
    assert_eq!(px(&p, 24, 0), BACKDROP);
    render(&mut p, 7);
    assert_eq!(px(&p, 16, 7), RED);
    assert_eq!(px(&p, 31, 7), RED);
}

#[test]
fn obj_palette_bank() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    obj(&mut p, 0, 0, 0, 1 | 1 << 12);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BLUE);
}

#[test]
fn obj_order_lower_index_wins_at_equal_priority_and_lower_prio_number_wins() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    obj(&mut p, 5, 0, 0, 2); // green, prio 0
    obj(&mut p, 9, 0, 0, 1); // red, prio 0
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
    obj(&mut p, 5, 0, 0, 2 | 1 << 10); // green now prio 1
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
}

#[test]
fn obj_priority_relative_to_bg() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    pal(&mut p, 1, BLUE);
    set16(&mut p, 0x00, 1 << 12 | 1 << 8);
    set16(&mut p, 0x08, bgcnt(1, 0, 8, 0, false)); // BG0 prio 1, blue
    tile4_solid(&mut p, 32, 1);
    vram16(&mut p, 0x4000, 1);
    obj(&mut p, 0, 1 << 14, 0, 1 | 1 << 10); // 16x8 OBJ (tiles 1 red, 2 green), prio 1 = BG -> OBJ on top
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    obj(&mut p, 0, 1 << 14, 0, 1 | 2 << 10); // OBJ prio 2: behind BG
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BLUE);
    assert_eq!(px(&p, 8, 0), GREEN, "BG transparent there -> OBJ visible");
}

#[test]
fn obj_affine_identity_and_scale() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    // Group 0: PA=PD=1.0 -> identity 8x8 at (0,0).
    oam16(&mut p, 0x06, 0x100);
    oam16(&mut p, 0x0E, 0);
    oam16(&mut p, 0x16, 0);
    oam16(&mut p, 0x1E, 0x100);
    obj(&mut p, 0, 1 << 8, 0, 1);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 7, 0), RED);
    assert_eq!(px(&p, 8, 0), BACKDROP);
    // PA=PD=2.0 shrinks the texture to the centre 4x4 of the 8x8 window.
    oam16(&mut p, 0x06, 0x200);
    oam16(&mut p, 0x1E, 0x200);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP);
    render(&mut p, 2);
    assert_eq!(px(&p, 1, 2), BACKDROP);
    assert_eq!(px(&p, 2, 2), RED);
    assert_eq!(px(&p, 5, 2), RED);
    assert_eq!(px(&p, 6, 2), BACKDROP);
}

#[test]
fn obj_affine_double_size() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    // PA=PD=0.5: texture magnified 2x. Normal 8x8 window shows the centre; double-size shows all 16x16.
    oam16(&mut p, 0x06, 0x80);
    oam16(&mut p, 0x0E, 0);
    oam16(&mut p, 0x16, 0);
    oam16(&mut p, 0x1E, 0x80);
    obj(&mut p, 0, 1 << 8, 0, 1);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 8, 0), BACKDROP);
    obj(&mut p, 0, 1 << 8 | 1 << 9, 0, 1);
    render(&mut p, 0);
    assert_eq!(px(&p, 15, 0), RED);
    assert_eq!(px(&p, 16, 0), BACKDROP);
    render(&mut p, 15);
    assert_eq!(px(&p, 15, 15), RED);
    render(&mut p, 16);
    assert_eq!(px(&p, 0, 16), BACKDROP);
}

#[test]
fn obj_affine_rotate_90() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    // 16x8 horizontal sprite of solid red rotated 90 degrees inside a double-size 32x16 box.
    // PA=0, PB=-1, PC=1, PD=0: screen (sx, sy) -> texel (8 - (sy - 8), (sx - 16) + 4).
    // Texel x is in 0..16 for sy 1..=15 (row 0 maps to texel x 16, outside: the classic one-pixel
    // shift of integer affine maths); texel y is in 0..8 for sx 12..=19.
    oam16(&mut p, 0x06, 0);
    oam16(&mut p, 0x0E, (-0x100i16) as u16);
    oam16(&mut p, 0x16, 0x100);
    oam16(&mut p, 0x1E, 0);
    set16(&mut p, 0x00, 1 << 12 | 1 << 6);
    obj(&mut p, 0, 1 << 8 | 1 << 9 | 1 << 14, 0, 1); // horizontal, size 0 = 16x8, double
    p.vram[0x10040..0x10060].fill(0x11); // tile 2 also red
    render(&mut p, 0);
    assert!(row(&p, 0).iter().all(|&c| c == BACKDROP));
    render(&mut p, 1);
    assert_eq!(px(&p, 11, 1), BACKDROP);
    assert_eq!(px(&p, 12, 1), RED);
    assert_eq!(px(&p, 19, 1), RED);
    assert_eq!(px(&p, 20, 1), BACKDROP);
    render(&mut p, 15);
    assert_eq!(px(&p, 12, 15), RED);
    assert_eq!(px(&p, 19, 15), RED);
    render(&mut p, 16);
    assert_eq!(px(&p, 12, 16), BACKDROP);
}

#[test]
fn obj_bitmap_mode_ignores_low_tiles() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    set16(&mut p, 0x00, 3 | 1 << 12);
    p.vram[0x14000..0x14020].fill(0x11); // tile 512
    obj(&mut p, 0, 0, 0, 1);
    obj(&mut p, 1, 0, 16, 512);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BACKDROP, "tile 1 not displayed in mode 3");
    assert_eq!(px(&p, 16, 0), RED);
}

#[test]
fn obj_window_masks_layers() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    pal(&mut p, 1, BLUE);
    set16(&mut p, 0x00, 1 << 12 | 1 << 8 | 1 << 15);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4_solid(&mut p, 32, 1);
    for i in 0..32 {
        vram16(&mut p, 0x4000 + i * 2, 1); // whole top row blue
    }
    obj(&mut p, 0, 2 << 10, 8, 1); // OBJ window at x 8..16
    obj(&mut p, 1, 0, 30, 2); // normal green OBJ at x 30
    set16(&mut p, 0x4A, 0x3F | (0x10 << 8)); // outside: all; OBJ window: only OBJ
    render(&mut p, 0);
    assert_eq!(px(&p, 7, 0), BLUE);
    assert_eq!(px(&p, 8, 0), BACKDROP, "BG0 masked inside OBJ window; window OBJ itself invisible");
    assert_eq!(px(&p, 15, 0), BACKDROP);
    assert_eq!(px(&p, 16, 0), BLUE);
    assert_eq!(px(&p, 30, 0), GREEN);
    // With DISPCNT bit 15 clear the OBJ-window sprite has no effect at all.
    set16(&mut p, 0x00, 1 << 12 | 1 << 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 8, 0), BLUE);
}

// ---------------------------------------------------------------- windows

/// BG0 blue everywhere (prio 0), OBJ green 8x8 at (100, 0).
fn window_setup(p: &mut Ppu) {
    obj_setup(p);
    hide_all_objs(p);
    pal(p, 1, BLUE);
    set16(p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4_solid(p, 32, 1);
    for i in 0..0x400 {
        vram16(p, 0x4000 + i * 2, 1);
    }
    obj(p, 0, 0, 100, 2);
}

#[test]
fn win0_inside_outside_masks() {
    let mut p = ppu();
    window_setup(&mut p);
    set16(&mut p, 0x00, 1 << 12 | 1 << 8 | 1 << 13);
    set16(&mut p, 0x40, 20 << 8 | 120); // x 20..120
    set16(&mut p, 0x44, 0 << 8 | 40); // y 0..40
    set16(&mut p, 0x48, 0x10); // inside: OBJ only
    set16(&mut p, 0x4A, 0x01); // outside: BG0 only
    render(&mut p, 0);
    assert_eq!(px(&p, 19, 0), BLUE);
    assert_eq!(px(&p, 20, 0), BACKDROP);
    assert_eq!(px(&p, 100, 0), GREEN);
    assert_eq!(px(&p, 119, 0), BACKDROP);
    assert_eq!(px(&p, 120, 0), BLUE);
    render(&mut p, 40);
    assert_eq!(px(&p, 50, 40), BLUE, "y 40 is outside (y2 exclusive)");
}

#[test]
fn win1_below_win0_and_inverted_x() {
    let mut p = ppu();
    window_setup(&mut p);
    set16(&mut p, 0x00, 1 << 8 | 1 << 13 | 1 << 14);
    set16(&mut p, 0x40, 10 << 8 | 20); // win0 x 10..20
    set16(&mut p, 0x44, 160);
    set16(&mut p, 0x42, 200 << 8 | 30); // win1 x1 > x2: covers x < 30 and x >= 200
    set16(&mut p, 0x46, 160);
    set16(&mut p, 0x48, 0x00 | (0x01 << 8)); // win0: nothing; win1: BG0
    set16(&mut p, 0x4A, 0x00); // outside: nothing
    render(&mut p, 5);
    assert_eq!(px(&p, 0, 5), BLUE, "win1 wraps to the left edge");
    assert_eq!(px(&p, 10, 5), BACKDROP, "win0 has priority over win1");
    assert_eq!(px(&p, 20, 5), BLUE);
    assert_eq!(px(&p, 29, 5), BLUE);
    assert_eq!(px(&p, 30, 5), BACKDROP);
    assert_eq!(px(&p, 199, 5), BACKDROP);
    assert_eq!(px(&p, 200, 5), BLUE);
    assert_eq!(px(&p, 239, 5), BLUE);
}

#[test]
fn window_inverted_y_and_clamp() {
    let mut p = ppu();
    window_setup(&mut p);
    set16(&mut p, 0x00, 1 << 8 | 1 << 13);
    set16(&mut p, 0x40, 0 << 8 | 0xFF); // x2 = 255 -> clamped to 240
    set16(&mut p, 0x44, 100 << 8 | 20); // y1 > y2: y >= 100 or y < 20
    set16(&mut p, 0x48, 0x01);
    set16(&mut p, 0x4A, 0x00);
    render(&mut p, 10);
    assert_eq!(px(&p, 239, 10), BLUE);
    render(&mut p, 20);
    assert_eq!(px(&p, 0, 20), BACKDROP);
    render(&mut p, 99);
    assert_eq!(px(&p, 0, 99), BACKDROP);
    render(&mut p, 100);
    assert_eq!(px(&p, 0, 100), BLUE);
}

#[test]
fn window_disables_effects() {
    let mut p = ppu();
    window_setup(&mut p);
    set16(&mut p, 0x00, 1 << 8 | 1 << 13);
    set16(&mut p, 0x40, 0 << 8 | 120);
    set16(&mut p, 0x44, 160);
    set16(&mut p, 0x48, 0x01); // inside: BG0, no effects
    set16(&mut p, 0x4A, 0x21); // outside: BG0 + effects
    set16(&mut p, 0x50, 0x01 | 3 << 6); // darken BG0
    set16(&mut p, 0x54, 16);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), BLUE);
    assert_eq!(px(&p, 120, 0), 0);
}

// ---------------------------------------------------------------- colour special effects

/// BG0 prio 0 at x 0..8 colour A (pal 1), BG1 prio 1 everywhere colour B (pal 2). Backdrop C.
fn blend_setup(p: &mut Ppu, a: u16, b: u16) {
    set16(p, 0x00, 1 << 8 | 1 << 9);
    pal(p, 0, BACKDROP);
    pal(p, 1, a);
    pal(p, 2, b);
    set16(p, 0x08, bgcnt(0, 0, 8, 0, false));
    set16(p, 0x0A, bgcnt(1, 0, 9, 0, false));
    tile4_solid(p, 32, 1);
    tile4_solid(p, 64, 2);
    vram16(p, 0x4000, 1);
    for i in 0..0x400 {
        vram16(p, 0x4800 + i * 2, 2);
    }
}

#[test]
fn alpha_blend_arithmetic_and_clamp() {
    let mut p = ppu();
    // A = (r 31, g 16, b 0), B = (r 31, g 8, b 4)
    let a = 31 | 16 << 5;
    let b = 31 | 8 << 5 | 4 << 10;
    blend_setup(&mut p, a, b);
    set16(&mut p, 0x50, 0x01 | 1 << 6 | 0x02 << 8); // BG0 1st, BG1 2nd, alpha
    set16(&mut p, 0x52, 8 | 8 << 8); // EVA = EVB = 8/16
    render(&mut p, 0);
    // r: (31*8 + 31*8)/16 = 31, g: (16*8 + 8*8)/16 = 12, b: (0 + 4*8)/16 = 2
    assert_eq!(px(&p, 0, 0), 31 | 12 << 5 | 2 << 10);
    assert_eq!(px(&p, 8, 0), b, "BG1 alone is not blended");
    // EVA 31 -> clamps to 16/16: r = min(31, 31 + 31*8/16) = 31, g = min(31, 16 + 4) = 20
    set16(&mut p, 0x52, 31 | 8 << 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 31 | 20 << 5 | 2 << 10);
    // EVA = EVB = 0 -> black
    set16(&mut p, 0x52, 0);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 0);
}

#[test]
fn alpha_requires_adjacent_second_target() {
    let mut p = ppu();
    blend_setup(&mut p, RED, GREEN);
    // Second target is backdrop only: BG1 sits between BG0 and backdrop -> no blend.
    set16(&mut p, 0x50, 0x01 | 1 << 6 | 0x20 << 8);
    set16(&mut p, 0x52, 8 | 8 << 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    // Disable BG1: now backdrop is directly below -> blend red with backdrop.
    set16(&mut p, 0x00, 1 << 8);
    render(&mut p, 0);
    let expect = {
        let r = (31 * 8 + (BACKDROP & 0x1F) as u32 * 8) >> 4;
        let g = ((BACKDROP >> 5 & 0x1F) as u32 * 8) >> 4;
        let bl = ((BACKDROP >> 10 & 0x1F) as u32 * 8) >> 4;
        (r | g << 5 | bl << 10) as u16
    };
    assert_eq!(px(&p, 0, 0), expect);
    // Second target above first target: no blend (BG1 prio 1 is 1st target, BG0 prio 0 is 2nd).
    set16(&mut p, 0x00, 1 << 8 | 1 << 9);
    set16(&mut p, 0x50, 0x02 | 1 << 6 | 0x01 << 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 8, 0), GREEN);
}

#[test]
fn brightness_increase_and_decrease() {
    let mut p = ppu();
    let a = 10 | 20 << 5 | 31 << 10;
    blend_setup(&mut p, a, GREEN);
    set16(&mut p, 0x50, 0x01 | 2 << 6);
    set16(&mut p, 0x54, 8); // EVY = 8/16
    render(&mut p, 0);
    // r: 10 + (21*8)/16 = 20, g: 20 + (11*8)/16 = 25, b: 31
    assert_eq!(px(&p, 0, 0), 20 | 25 << 5 | 31 << 10);
    assert_eq!(px(&p, 8, 0), GREEN, "BG1 is not a 1st target");
    set16(&mut p, 0x50, 0x01 | 3 << 6);
    render(&mut p, 0);
    // r: 10 - 5 = 5, g: 20 - 10 = 10, b: 31 - 15 = 16
    assert_eq!(px(&p, 0, 0), 5 | 10 << 5 | 16 << 10);
    set16(&mut p, 0x54, 31); // clamps to 16 -> black
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 0);
    set16(&mut p, 0x50, 0x01 | 2 << 6);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), WHITE);
}

#[test]
fn backdrop_as_first_target() {
    let mut p = ppu();
    pal(&mut p, 0, 0x7FFF);
    set16(&mut p, 0x50, 0x20 | 3 << 6);
    set16(&mut p, 0x54, 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 16 | 16 << 5 | 16 << 10);
}

#[test]
fn semi_transparent_obj_forces_alpha() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    pal(&mut p, 1, BLUE);
    pal(&mut p, 256 + 1, RED);
    set16(&mut p, 0x00, 1 << 12 | 1 << 8);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4_solid(&mut p, 32, 1);
    vram16(&mut p, 0x4000, 1); // BG0 blue at x 0..8
    obj(&mut p, 0, 1 << 10, 0, 1); // semi-transparent red OBJ over it, extends to x 8..? no: 8x8 at x 0
    obj(&mut p, 1, 1 << 10, 8, 1); // second semi OBJ over backdrop at x 8..16
    set16(&mut p, 0x52, 8 | 8 << 8);
    // BLDCNT: mode brightness-decrease with OBJ as 1st target, BG0 as 2nd target, backdrop not.
    set16(&mut p, 0x50, 0x10 | 3 << 6 | 0x01 << 8);
    set16(&mut p, 0x54, 16);
    render(&mut p, 0);
    // Over BG0 (2nd target): alpha wins over brightness: r 31*8/16 = 15, b 31*8/16 = 15.
    assert_eq!(px(&p, 0, 0), 15 | 15 << 10);
    // Over backdrop (not a 2nd target): brightness decrease applies (OBJ is 1st target) -> black.
    assert_eq!(px(&p, 8, 0), 0);
    // BLDCNT mode 0 and OBJ not a 1st target: semi-transparency still blends over a 2nd target.
    set16(&mut p, 0x50, 0x01 << 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 15 | 15 << 10);
    assert_eq!(px(&p, 8, 0), RED);
}

#[test]
fn obj_blend_with_bg_normal_mode() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    pal(&mut p, 1, BLUE);
    set16(&mut p, 0x00, 1 << 12 | 1 << 8);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4_solid(&mut p, 32, 1);
    vram16(&mut p, 0x4000, 1);
    obj(&mut p, 0, 0, 0, 1);
    set16(&mut p, 0x50, 0x10 | 1 << 6 | 0x01 << 8);
    set16(&mut p, 0x52, 16 | 16 << 8);
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 31 | 31 << 10, "red + blue at full weights");
}

// ---------------------------------------------------------------- mosaic

#[test]
fn bg_mosaic_horizontal_and_vertical() {
    let mut p = ppu();
    basic_mode0(&mut p);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false) | 1 << 6);
    // Tile 1: pixel (x,y) = 1 if x == y else 0 (diagonal).
    tile4(&mut p, 32, |x, y| (x == y) as u8);
    vram16(&mut p, 0x4000, 1);
    set16(&mut p, 0x4C, 3 | 3 << 4); // 4x4 mosaic
    render(&mut p, 0);
    // Line 0 = source line 0: pixel 0 set -> pixels 0..4 red; pixel 4..8 from pixel 4 (0 on line 0) -> backdrop.
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 3, 0), RED);
    assert_eq!(px(&p, 4, 0), BACKDROP);
    render(&mut p, 2);
    assert_eq!(px(&p, 0, 2), RED, "line 2 shows source line 0");
    render(&mut p, 5);
    // Source line 4: pixel 4 set -> pixels 4..8 red, 0..4 backdrop.
    assert_eq!(px(&p, 0, 5), BACKDROP);
    assert_eq!(px(&p, 4, 5), RED);
    assert_eq!(px(&p, 7, 5), RED);
    assert_eq!(px(&p, 8, 5), BACKDROP);
    // Mosaic off in BGCNT -> unaffected.
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    render(&mut p, 5);
    assert_eq!(px(&p, 4, 5), BACKDROP);
    assert_eq!(px(&p, 5, 5), RED);
}

#[test]
fn affine_bg_mosaic_vertical_backs_up_reference() {
    let mut p = ppu();
    affine_setup(&mut p, false);
    set16(&mut p, 0x0C, bgcnt(0, 0, 8, 0, true) | 1 << 6);
    set16(&mut p, 0x4C, 0 | 3 << 4); // v = 4
    // Tile row 0 is red for texel y 0..8; with v=4 lines 8..11 show line 8 (empty), lines 4..7 show line 4 (red).
    for y in 0..12 {
        render(&mut p, y);
        let expect = if y < 8 { RED } else { BACKDROP };
        assert_eq!(px(&p, 0, y), expect, "line {y}");
    }
}

#[test]
fn obj_mosaic() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    // Tile 3: diagonal.
    p.vram[0x10060..0x10080].fill(0);
    for i in 0..8 {
        let b = &mut p.vram[0x10060 + i * 4 + i / 2];
        *b |= if i & 1 == 0 { 0x01 } else { 0x10 };
    }
    obj(&mut p, 0, 1 << 12, 0, 3); // mosaic on, at (0,0)
    set16(&mut p, 0x4C, (3 | 3 << 4) << 8); // OBJ 4x4
    render(&mut p, 1);
    assert_eq!(px(&p, 0, 1), RED, "source (0,0)");
    assert_eq!(px(&p, 3, 1), RED);
    assert_eq!(px(&p, 4, 1), BACKDROP);
    render(&mut p, 6);
    assert_eq!(px(&p, 4, 6), RED, "source (4,4)");
    assert_eq!(px(&p, 3, 6), BACKDROP);
    // Sprite at x 2: block starting at x 0 samples outside the sprite -> transparent for x 0..4.
    obj(&mut p, 0, 1 << 12, 2, 3);
    render(&mut p, 0);
    assert_eq!(px(&p, 2, 0), BACKDROP);
    assert_eq!(px(&p, 4, 0), BACKDROP, "block 4..8 samples sprite x 2 (pixel (2,0) is 0)");
}

// ---------------------------------------------------------------- green swap

#[test]
fn green_swap_exchanges_pairs() {
    let mut p = ppu();
    set16(&mut p, 0x00, 3 | 1 << 10);
    set16(&mut p, 0x20, 0x100);
    set16(&mut p, 0x26, 0x100);
    vram16(&mut p, 0, 1 | 2 << 5 | 3 << 10);
    vram16(&mut p, 2, 4 | 5 << 5 | 6 << 10);
    frame_line(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 1 | 2 << 5 | 3 << 10);
    set16(&mut p, 0x02, 1);
    frame_line(&mut p, 0);
    assert_eq!(px(&p, 0, 0), 1 | 5 << 5 | 3 << 10);
    assert_eq!(px(&p, 1, 0), 4 | 2 << 5 | 6 << 10);
}

// ---------------------------------------------------------------- IO access

#[test]
fn write_only_registers_read_zero_and_dispcnt_reads_back() {
    let mut p = ppu();
    set16(&mut p, 0x00, 0x1234);
    set16(&mut p, 0x10, 0x00FF);
    set16(&mut p, 0x4C, 0x00FF);
    set16(&mut p, 0x50, 0x3F3F);
    assert_eq!(p.read_io8(0x00), 0x34);
    assert_eq!(p.read_io8(0x01), 0x12);
    assert_eq!(p.read_io8(0x10), 0);
    assert_eq!(p.read_io8(0x4C), 0);
    assert_eq!(p.read_io8(0x50), 0x3F);
}

#[test]
fn whole_frame_output_has_bit15_clear() {
    let mut p = ppu();
    set16(&mut p, 0x00, 3 | 1 << 10);
    set16(&mut p, 0x20, 0x100);
    set16(&mut p, 0x26, 0x100);
    for i in 0..240 * 160 {
        vram16(&mut p, i * 2, 0xFFFF);
    }
    for y in 0..160 {
        render(&mut p, y);
    }
    assert!(p.fb.iter().all(|&c| c == 0x7FFF));
}

// ---------------------------------------------------------------- OBJ cycle budget (GBATEK "LCD OBJ Overview")

/// OBJ on, 1D mapping, every OBJ tile solid colour 1 (red), backdrop set.
fn budget_setup(p: &mut Ppu, hblank_free: bool) {
    pal(p, 0, BACKDROP);
    pal(p, 256 + 1, RED);
    pal(p, 256 + 2, GREEN);
    set16(p, 0x00, (1 << 12) | (1 << 6) | if hblank_free { 1 << 5 } else { 0 });
    p.vram[0x10000..0x18000].fill(0x11);
    hide_all_objs(p);
    // Affine parameter group 0 = identity.
    oam16(p, 0x06, 0x100);
    oam16(p, 0x1E, 0x100);
}

#[test]
fn obj_budget_128_small_sprites_all_fit() {
    let mut p = ppu();
    budget_setup(&mut p, false);
    for i in 0..127 {
        obj(&mut p, i, 0, 0, 1); // 127 x 8 px = 1016 cycles
    }
    p.vram[0x10040..0x10060].fill(0x22); // tile 2 = green
    obj(&mut p, 127, 0, 200, 2); // 128th sprite: 1024 <= 1210, still drawn
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), RED);
    assert_eq!(px(&p, 200, 0), GREEN);
}

/// Four double-size 64x64 affine sprites (266 cycles each) placed off-screen to the right still
/// eat 1064 cycles; the fifth gets the remaining 146 = 10 + 2*68 -> only its first 68 columns.
#[test]
fn obj_budget_affine_partial_draw_and_cutoff() {
    let mut p = ppu();
    budget_setup(&mut p, false);
    let a0 = 0 | (1 << 8) | (1 << 9); // y 0, affine, double size
    for i in 0..4 {
        obj(&mut p, i, a0, 240 | (3 << 14), 1); // x = 240: horizontally offscreen
    }
    obj(&mut p, 4, a0, 3 << 14, 1); // x = 0, 128 px wide on screen
    p.vram[0x10040..0x10060].fill(0x22);
    obj(&mut p, 5, 64, 150, 2); // regular 8x8 green at (150, 64): no budget left
    render(&mut p, 64);
    // Identity affine, double size: texels cover screen x 32..96 on the middle line.
    assert_eq!(px(&p, 32, 64), RED);
    assert_eq!(px(&p, 67, 64), RED);
    assert_eq!(px(&p, 68, 64), BACKDROP, "budget ends after column 68");
    assert_eq!(px(&p, 95, 64), BACKDROP);
    assert_eq!(px(&p, 150, 64), BACKDROP, "later sprites are not rendered");
    // Without the four blockers the sprite is complete and the small one renders.
    for i in 0..4 {
        oam16(&mut p, i * 8, 1 << 9);
    }
    render(&mut p, 64);
    assert_eq!(px(&p, 95, 64), RED);
    assert_eq!(px(&p, 150, 64), GREEN);
}

/// H-Blank Interval Free shrinks the budget to 954: only three 266-cycle sprites fit, the fourth
/// blocker exhausts it, and the on-screen fifth sprite never renders.
#[test]
fn obj_budget_hblank_interval_free() {
    let mut p = ppu();
    budget_setup(&mut p, true);
    let a0 = 0 | (1 << 8) | (1 << 9);
    for i in 0..4 {
        obj(&mut p, i, a0, 240 | (3 << 14), 1);
    }
    obj(&mut p, 4, a0, 3 << 14, 1);
    render(&mut p, 64);
    assert_eq!(px(&p, 40, 64), BACKDROP);
    // With three blockers: 798 used, 156 left = 10 + 2*73 -> 73 columns of the fifth sprite.
    oam16(&mut p, 3 * 8, 1 << 9);
    render(&mut p, 64);
    assert_eq!(px(&p, 72, 64), RED);
    assert_eq!(px(&p, 73, 64), BACKDROP);
}

/// Sprites that are not on this line cost nothing.
#[test]
fn obj_budget_ignores_vertically_offscreen_sprites() {
    let mut p = ppu();
    budget_setup(&mut p, false);
    let a0 = 100 | (1 << 8) | (1 << 9); // y 100: rows 100..227, not on line 0
    for i in 0..8 {
        obj(&mut p, i, a0, 3 << 14, 1);
    }
    obj(&mut p, 8, 0, 10, 1);
    render(&mut p, 0);
    assert_eq!(px(&p, 10, 0), RED);
}
