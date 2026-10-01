//! Bench capture: per-layer pictures of the frame being rendered, for the developer Bench view.
//!
//! The PPU owns a `PpuCapture` behind `Ppu::debug` (`None` unless the bench is attached via
//! `Bus::set_bench`). `render_scanline` copies each layer's raw per-line buffer into `cur`
//! before windows, priority and blending are applied; the Bus swaps `cur` into the published
//! `LayerFrame` once per frame (`Bus::bench_end_frame`). Nothing here is serialised.
//!
//! Layer planes hold BGR555 colours with bit 15 set meaning "transparent" (the same encoding the
//! per-line scratch buffers use), so a plane is exactly what the layer alone would show.

use super::obj::ObjLine;
use super::regs::TRANSPARENT;
use crate::{SCREEN_H, SCREEN_W};

pub const LAYER_BG0: usize = 0;
pub const LAYER_BG1: usize = 1;
pub const LAYER_BG2: usize = 2;
pub const LAYER_BG3: usize = 3;
pub const LAYER_OBJ: usize = 4;

/// One layer's picture, row-major, `SCREEN_W * SCREEN_H` BGR555 entries.
pub type LayerPlane = [u16; SCREEN_W * SCREEN_H];

/// Per-layer pictures of one frame: BGR555, bit 15 set = transparent. This is the raw layer
/// output, before windows, priority and blending.
#[derive(Clone)]
pub struct LayerFrame {
    /// Planes 0-3 are BG0-BG3, plane 4 is OBJ.
    pub planes: Box<[LayerPlane; 5]>,
    /// Bit i set: layer i was displayed on at least one line of the frame (bit 4 = OBJ).
    pub enabled: u8,
    /// Bit k of word w: OAM entry `w * 64 + k` drew at least one pixel into the OBJ line buffer.
    pub obj_seen: [u64; 2],
    /// Number of distinct sprites that drew at least one pixel this frame.
    pub obj_drawn: u8,
    /// Number of OAM entries that are not disabled and have a legal shape (set at frame end).
    pub obj_active: u8,
}

impl Default for LayerFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerFrame {
    /// All planes transparent, nothing enabled.
    pub fn new() -> Self {
        let planes: Box<[LayerPlane; 5]> = vec![[TRANSPARENT; SCREEN_W * SCREEN_H]; 5].into_boxed_slice().try_into().unwrap();
        LayerFrame { planes, enabled: 0, obj_seen: [0; 2], obj_drawn: 0, obj_active: 0 }
    }

    /// Start a new frame: clears the per-frame flags; the planes are overwritten line by line.
    pub fn reset_frame(&mut self) {
        self.enabled = 0;
        self.obj_seen = [0; 2];
    }
}

/// Capture state owned by the PPU while the bench is attached.
pub struct PpuCapture {
    /// The frame currently being rendered.
    pub cur: Box<LayerFrame>,
    /// Layers hidden from the composed output: bits 0-3 BG0-BG3, bit 4 OBJ. Capture is unaffected.
    pub mute: u8,
    /// An OBJ line with no pixels, substituted for the real one when OBJ is muted.
    pub(super) empty_obj: Box<ObjLine>,
}

impl Default for PpuCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl PpuCapture {
    pub fn new() -> Self {
        PpuCapture { cur: Box::new(LayerFrame::new()), mute: 0, empty_obj: Box::new(ObjLine::new()) }
    }

    /// Record line `y` from the raw per-layer buffers. A layer that is not displayed on this
    /// line is recorded as transparent.
    pub(super) fn capture_line(&mut self, y: usize, bg: &[[u16; SCREEN_W]; 4], bg_active: [bool; 4], obj: &ObjLine, obj_on: bool) {
        let cur = &mut *self.cur;
        let range = y * SCREEN_W..(y + 1) * SCREEN_W;
        let mut en = 0u8;
        for i in 0..4 {
            let dst = &mut cur.planes[i][range.clone()];
            if bg_active[i] {
                dst.copy_from_slice(&bg[i]);
                en |= 1 << i;
            } else {
                dst.fill(TRANSPARENT);
            }
        }
        let dst = &mut cur.planes[LAYER_OBJ][range];
        if obj_on {
            dst.copy_from_slice(&obj.color);
            en |= 1 << LAYER_OBJ;
        } else {
            dst.fill(TRANSPARENT);
        }
        cur.enabled |= en;
        cur.obj_seen[0] |= obj.drawn[0];
        cur.obj_seen[1] |= obj.drawn[1];
    }

    /// Record a forced-blank line: every plane transparent.
    pub(super) fn blank_line(&mut self, y: usize) {
        let range = y * SCREEN_W..(y + 1) * SCREEN_W;
        for plane in self.cur.planes.iter_mut() {
            plane[range.clone()].fill(TRANSPARENT);
        }
    }
}
