//! Bench capture: per-layer planes, mid-frame changes, forced blank, layer mutes, sprite counting.

use super::*;
use crate::ppu::capture::{PpuCapture, LAYER_BG0, LAYER_OBJ};

const T: u16 = TRANSPARENT;

fn attach(p: &mut Ppu) {
    p.debug = Some(Box::new(PpuCapture::new()));
}

fn cap(p: &Ppu) -> &PpuCapture {
    p.debug.as_deref().unwrap()
}

fn plane(p: &Ppu, layer: usize, x: usize, y: usize) -> u16 {
    cap(p).cur.planes[layer][y * W + x]
}

/// Mode 0 with BG0 (red, prio 0, x 0..8), BG1 (green, prio 1, x 0..16) and OBJ 0 (blue 8x8 at
/// (100, 0)) on, everything else transparent.
fn scene(p: &mut Ppu) {
    obj_setup(p);
    hide_all_objs(p);
    p.vram[0x1_0060..0x1_0080].fill(0x33); // OBJ tile 3 = solid 3 (blue)
    pal(p, 1, RED);
    pal(p, 2, GREEN);
    set16(p, 0x00, 1 << 8 | 1 << 9 | 1 << 12);
    set16(p, 0x08, bgcnt(0, 0, 8, 0, false));
    set16(p, 0x0A, bgcnt(1, 0, 9, 0, false));
    tile4_solid(p, 32, 1);
    tile4_solid(p, 64, 2);
    vram16(p, 0x4000, 1);
    vram16(p, 0x4800, 2);
    vram16(p, 0x4802, 2);
    obj(p, 0, 0, 100, 3);
}

#[test]
fn capture_off_is_default_and_output_identical() {
    assert!(ppu().debug.is_none());
    let mut off = ppu();
    let mut on = ppu();
    attach(&mut on);
    scene(&mut off);
    scene(&mut on);
    for y in 0..12 {
        render(&mut off, y);
        render(&mut on, y);
    }
    assert_eq!(off.fb, on.fb);
    assert_eq!(px(&on, 0, 0), RED);
    assert_eq!(px(&on, 100, 0), BLUE);
}

#[test]
fn capture_planes_hold_raw_layer_colours() {
    let mut p = ppu();
    scene(&mut p);
    attach(&mut p);
    for y in 0..8 {
        render(&mut p, y);
    }
    for y in 0..8 {
        for x in 0..W {
            let (bg0, bg1, obj) = (plane(&p, LAYER_BG0, x, y), plane(&p, 1, x, y), plane(&p, LAYER_OBJ, x, y));
            assert_eq!(bg0, if x < 8 { RED } else { T }, "BG0 ({x},{y})");
            assert_eq!(bg1, if x < 16 { GREEN } else { T }, "BG1 is raw, not occluded by BG0 ({x},{y})");
            assert_eq!(obj, if (100..108).contains(&x) { BLUE } else { T }, "OBJ ({x},{y})");
            assert_eq!(plane(&p, 2, x, y), T);
            assert_eq!(plane(&p, 3, x, y), T);
        }
    }
    assert_eq!(px(&p, 0, 0), RED, "composed output still shows BG0 on top");
    assert_eq!(cap(&p).cur.enabled, 0b1_0011);
    assert!(cap(&p).cur.planes[2].iter().all(|&c| c == T));
    assert!(cap(&p).cur.planes[3].iter().all(|&c| c == T));
}

#[test]
fn capture_follows_mid_frame_dispcnt_changes() {
    let mut p = ppu();
    scene(&mut p);
    vram16(&mut p, 0x4000 + 32 * 2, 1); // BG0 tile row 1 too, so lines 8..=19 have content
    vram16(&mut p, 0x4000 + 64 * 2, 1);
    attach(&mut p);
    for y in 0..20 {
        // BG0 on for lines 0..=9, off for 10..=19; OBJ/BG1 stay off the whole time.
        set16(&mut p, 0x00, if y < 10 { 1 << 8 } else { 0 });
        render(&mut p, y);
    }
    assert_eq!(plane(&p, LAYER_BG0, 0, 9), RED);
    assert_eq!(plane(&p, LAYER_BG0, 0, 10), T, "BG0 switched off at line 10");
    assert_eq!(plane(&p, LAYER_BG0, 0, 19), T);
    assert_eq!(plane(&p, 1, 0, 0), T, "BG1 was never displayed");
    assert_eq!(plane(&p, LAYER_OBJ, 100, 0), T, "OBJ was never displayed");
    assert_eq!(cap(&p).cur.enabled, 0b0_0001, "enabled = displayed on at least one line");
}

#[test]
fn forced_blank_line_is_transparent_in_every_plane() {
    let mut p = ppu();
    scene(&mut p);
    attach(&mut p);
    render(&mut p, 0);
    assert_eq!(plane(&p, LAYER_BG0, 0, 0), RED);
    assert_eq!(plane(&p, LAYER_OBJ, 100, 0), BLUE);
    let dispcnt = p.dispcnt();
    set16(&mut p, 0x00, dispcnt | 0x80);
    render(&mut p, 0);
    assert!(row(&p, 0).iter().all(|&c| c == WHITE));
    for layer in 0..5 {
        assert!(cap(&p).cur.planes[layer][..W].iter().all(|&c| c == T), "plane {layer}");
    }
}

#[test]
fn layer_mute_removes_layer_from_output_not_from_capture() {
    let mut p = ppu();
    scene(&mut p);
    attach(&mut p);
    render(&mut p, 0);
    assert_eq!((px(&p, 0, 0), px(&p, 100, 0)), (RED, BLUE));

    // Mute BG0: BG1 shows through, the BG0 plane is still recorded.
    p.debug.as_deref_mut().unwrap().mute = 1 << 0;
    render(&mut p, 0);
    assert_eq!(px(&p, 0, 0), GREEN);
    assert_eq!(px(&p, 12, 0), GREEN);
    assert_eq!(plane(&p, LAYER_BG0, 0, 0), RED);
    assert_eq!(px(&p, 100, 0), BLUE, "other layers are untouched");

    // Mute OBJ as well.
    p.debug.as_deref_mut().unwrap().mute = 1 << 0 | 1 << 4;
    render(&mut p, 0);
    assert_eq!(px(&p, 100, 0), BACKDROP);
    assert_eq!(plane(&p, LAYER_OBJ, 100, 0), BLUE);

    // Mute everything: only the backdrop is left.
    p.debug.as_deref_mut().unwrap().mute = 0x1F;
    render(&mut p, 0);
    assert!(row(&p, 0).iter().all(|&c| c == BACKDROP));
    assert_eq!(plane(&p, 1, 0, 0), GREEN);

    // Unmuting restores the picture.
    p.debug.as_deref_mut().unwrap().mute = 0;
    render(&mut p, 0);
    assert_eq!((px(&p, 0, 0), px(&p, 100, 0)), (RED, BLUE));
}

#[test]
fn layer_mute_keeps_the_obj_window_working() {
    // Same scene as `obj_window_masks_layers`: BG0 blue along the top row, an OBJ-window sprite
    // over x 8..16 and a normal green sprite at x 30. WINOUT shows everything, the OBJ window
    // shows only OBJ.
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    pal(&mut p, 1, BLUE);
    set16(&mut p, 0x00, 1 << 12 | 1 << 8 | 1 << 15);
    set16(&mut p, 0x08, bgcnt(0, 0, 8, 0, false));
    tile4_solid(&mut p, 32, 1);
    for i in 0..32 {
        vram16(&mut p, 0x4000 + i * 2, 1);
    }
    obj(&mut p, 0, 2 << 10, 8, 1);
    obj(&mut p, 1, 0, 30, 2);
    set16(&mut p, 0x4A, 0x3F | (0x10 << 8));
    attach(&mut p);
    render(&mut p, 0);
    assert_eq!((px(&p, 7, 0), px(&p, 8, 0), px(&p, 30, 0)), (BLUE, BACKDROP, GREEN));

    p.debug.as_deref_mut().unwrap().mute = 1 << 4;
    render(&mut p, 0);
    assert_eq!(px(&p, 7, 0), BLUE);
    assert_eq!(px(&p, 8, 0), BACKDROP, "the OBJ window still masks BG0 while OBJ is muted");
    assert_eq!(px(&p, 15, 0), BACKDROP);
    assert_eq!(px(&p, 16, 0), BLUE);
    assert_eq!(px(&p, 30, 0), BLUE, "the muted normal sprite is gone (BG0 shows)");
    assert_eq!(plane(&p, LAYER_OBJ, 30, 0), GREEN, "but it is still captured");
    assert_eq!(plane(&p, LAYER_OBJ, 8, 0), T, "OBJ-window sprites draw no colour");
}

#[test]
fn obj_drawn_counts_distinct_sprites_once_per_frame() {
    let mut p = ppu();
    obj_setup(&mut p);
    hide_all_objs(&mut p);
    attach(&mut p);
    obj(&mut p, 0, 0, 10, 1); // visible
    obj(&mut p, 1, 0, 20, 2); // visible
    obj(&mut p, 70, 0, 30, 1); // visible, second word of the bitmap
    obj(&mut p, 5, 0, 50, 0); // tile 0 is blank: draws nothing
    obj(&mut p, 6, 100, 60, 1); // never on lines 0..8
    obj(&mut p, 7, 0, 240, 1); // off the right edge
    for y in 0..8 {
        render(&mut p, y);
    }
    let seen = cap(&p).cur.obj_seen;
    assert_eq!(seen[0], 0b11, "sprites 0 and 1, once although they span eight lines");
    assert_eq!(seen[1], 1 << 6, "sprite 70");
    assert_eq!(seen[0].count_ones() + seen[1].count_ones(), 3);
    // And the per-frame reset clears it for the next frame.
    p.debug.as_deref_mut().unwrap().cur.reset_frame();
    assert_eq!(cap(&p).cur.obj_seen, [0; 2]);
    assert_eq!(cap(&p).cur.enabled, 0);
}
