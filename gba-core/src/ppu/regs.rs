//! Typed views over the raw LCD register bytes (`Ppu::io`, offsets 0x000-0x05F).
//! Source: GBATEK "LCD I/O Display Control", "BG Control", "BG Scrolling", "BG Rotation/Scaling",
//! "Window Feature", "Mosaic Function", "Color Special Effects".

use super::{OAM_SIZE, PAL_SIZE, VRAM_SIZE};

/// Marker for a transparent pixel in a layer line buffer (bit 15 is never set on a real colour).
pub const TRANSPARENT: u16 = 0x8000;

/// Borrowed view of the memories the renderer reads. Lets the render functions run while the
/// scratch buffers in `Ppu` are mutably borrowed.
#[derive(Clone, Copy)]
pub struct Mem<'a> {
    pub io: &'a [u8; 0x60],
    pub pal: &'a [u8; PAL_SIZE],
    pub vram: &'a [u8; VRAM_SIZE],
    pub oam: &'a [u8; OAM_SIZE],
}

impl<'a> Mem<'a> {
    #[inline]
    pub fn reg16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.io[off], self.io[off + 1]])
    }

    #[inline]
    pub fn dispcnt(&self) -> u16 {
        self.reg16(0x00)
    }

    /// Palette entry `idx` (0..256 = BG, 256..512 = OBJ) as BGR555 with bit 15 clear.
    #[inline]
    pub fn pal_color(&self, idx: usize) -> u16 {
        let i = idx * 2;
        u16::from_le_bytes([self.pal[i], self.pal[i + 1]]) & 0x7FFF
    }

    #[inline]
    pub fn vram16(&self, addr: usize) -> u16 {
        u16::from_le_bytes([self.vram[addr], self.vram[addr + 1]])
    }

    #[inline]
    pub fn oam16(&self, addr: usize) -> u16 {
        u16::from_le_bytes([self.oam[addr], self.oam[addr + 1]])
    }

    pub fn bgcnt(&self, bg: usize) -> BgCnt {
        BgCnt::from_raw(self.reg16(0x08 + bg * 2))
    }

    /// BGxHOFS / BGxVOFS (9 bits each).
    pub fn bg_scroll(&self, bg: usize) -> (u32, u32) {
        (self.reg16(0x10 + bg * 4) as u32 & 0x1FF, self.reg16(0x12 + bg * 4) as u32 & 0x1FF)
    }

    /// PA, PB, PC, PD for BG2 (`bg == 2`) or BG3 (`bg == 3`), sign-extended 8.8 fixed point.
    pub fn bg_affine_params(&self, bg: usize) -> [i32; 4] {
        let base = if bg == 2 { 0x20 } else { 0x30 };
        [
            self.reg16(base) as i16 as i32,
            self.reg16(base + 2) as i16 as i32,
            self.reg16(base + 4) as i16 as i32,
            self.reg16(base + 6) as i16 as i32,
        ]
    }

    pub fn mosaic(&self) -> Mosaic {
        let v = self.reg16(0x4C);
        Mosaic {
            bg_h: (v & 0xF) as usize + 1,
            bg_v: ((v >> 4) & 0xF) as usize + 1,
            obj_h: ((v >> 8) & 0xF) as usize + 1,
            obj_v: ((v >> 12) & 0xF) as usize + 1,
        }
    }

    pub fn blend(&self) -> Blend {
        let cnt = self.reg16(0x50);
        let alpha = self.reg16(0x52);
        let y = self.reg16(0x54);
        Blend {
            target1: (cnt & 0x3F) as u8,
            target2: ((cnt >> 8) & 0x3F) as u8,
            mode: ((cnt >> 6) & 3) as u8,
            eva: ((alpha & 0x1F) as u32).min(16),
            evb: (((alpha >> 8) & 0x1F) as u32).min(16),
            evy: ((y & 0x1F) as u32).min(16),
        }
    }

    /// WIN0H/WIN0V (`n == 0`) or WIN1H/WIN1V (`n == 1`).
    pub fn window(&self, n: usize) -> Window {
        let h = self.reg16(0x40 + n * 2);
        let v = self.reg16(0x44 + n * 2);
        Window { x1: (h >> 8) as u32, x2: (h & 0xFF) as u32, y1: (v >> 8) as u32, y2: (v & 0xFF) as u32 }
    }

    /// WININ low/high byte, WINOUT low/high byte: layer masks (bits 0-3 BG, 4 OBJ, 5 effects).
    pub fn win_in(&self, n: usize) -> u8 {
        self.io[0x48 + n] & 0x3F
    }
    pub fn win_out(&self) -> u8 {
        self.io[0x4A] & 0x3F
    }
    pub fn win_obj(&self) -> u8 {
        self.io[0x4B] & 0x3F
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BgCnt {
    pub prio: u8,
    /// Byte offset into VRAM of the tile data (units of 16 KiB).
    pub char_base: usize,
    pub mosaic: bool,
    /// 256 colours / 1 palette when set; 16/16 otherwise.
    pub bpp8: bool,
    /// Byte offset into VRAM of the map data (units of 2 KiB).
    pub screen_base: usize,
    /// Display area overflow: wraparound (BG2/BG3 affine only).
    pub wrap: bool,
    pub size: u8,
}

impl BgCnt {
    pub fn from_raw(v: u16) -> Self {
        BgCnt {
            prio: (v & 3) as u8,
            char_base: ((v >> 2) & 3) as usize * 0x4000,
            mosaic: v & (1 << 6) != 0,
            bpp8: v & (1 << 7) != 0,
            screen_base: ((v >> 8) & 0x1F) as usize * 0x800,
            wrap: v & (1 << 13) != 0,
            size: (v >> 14) as u8,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mosaic {
    /// Sizes are already `+1`, i.e. 1..=16.
    pub bg_h: usize,
    pub bg_v: usize,
    pub obj_h: usize,
    pub obj_v: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Blend {
    /// Bits 0-3 BG0-3, 4 OBJ, 5 backdrop.
    pub target1: u8,
    pub target2: u8,
    /// 0 none, 1 alpha, 2 brightness increase, 3 brightness decrease.
    pub mode: u8,
    /// Coefficients already clamped to 0..=16.
    pub eva: u32,
    pub evb: u32,
    pub evy: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub x1: u32,
    pub x2: u32,
    pub y1: u32,
    pub y2: u32,
}

/// Layer identifiers used by the compositor and BLDCNT target bits.
pub const LAYER_OBJ: u8 = 4;
pub const LAYER_BACKDROP: u8 = 5;
