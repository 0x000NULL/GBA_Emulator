//! BIOS: LLE from a 16 KiB image when supplied, otherwise HLE of the SWI functions.
//! Source: GBATEK "GBA BIOS Functions", "BIOS RAM Usage", "Unpredictable Things" (BIOS reads).

pub mod hle;
pub mod swi_timing;

use crate::CoreError;

pub const BIOS_SIZE: usize = 0x4000;

/// Open-bus values returned when reading the BIOS region from outside it (GBATEK):
/// after startup E129F000h, after SWI E3A02004h, during IRQ E25EF004h, after IRQ E55EC002h.
pub const BIOS_OPENBUS_BOOT: u32 = 0xE129_F000;
pub const BIOS_OPENBUS_AFTER_SWI: u32 = 0xE3A0_2004;
pub const BIOS_OPENBUS_DURING_IRQ: u32 = 0xE25E_F004;
pub const BIOS_OPENBUS_AFTER_IRQ: u32 = 0xE55E_C002;

/// Address inside the BIOS region that the HLE IRQ dispatcher uses as the user handler's return
/// address; executing "from" it triggers the return sequence.
pub const HLE_IRQ_RETURN_ADDR: u32 = 0x0000_0138;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Bios {
    #[cfg_attr(feature = "savestate", serde(skip))]
    image: Option<Box<[u8; BIOS_SIZE]>>,
    /// Value returned for BIOS-region reads while PC is outside the BIOS.
    pub open_bus: u32,
    /// Pending IntrWait: flags being waited for; set while the HLE loop is parked on the SWI.
    pub intr_wait: Option<u16>,
    /// Counts SWIs serviced (diagnostics).
    pub swi_count: u64,
    /// Unimplemented SWI numbers seen (diagnostics), deduplicated.
    pub unimplemented: Vec<u8>,
    /// Set by an HLE SWI that modelled the real handler's full cycle cost: the CPU then refetches
    /// the pipeline at the return address (the `movs pc, lr` of the real handler).
    #[cfg_attr(feature = "savestate", serde(skip))]
    pub swi_refill: bool,
}

impl Bios {
    pub fn hle() -> Self {
        Bios { image: None, open_bus: BIOS_OPENBUS_BOOT, intr_wait: None, swi_count: 0, unimplemented: Vec::new(), swi_refill: false }
    }

    pub fn lle(data: &[u8]) -> Result<Self, CoreError> {
        if data.len() != BIOS_SIZE {
            return Err(CoreError::BadBios(data.len()));
        }
        let mut img = Box::new([0u8; BIOS_SIZE]);
        img.copy_from_slice(data);
        Ok(Bios { image: Some(img), open_bus: BIOS_OPENBUS_BOOT, intr_wait: None, swi_count: 0, unimplemented: Vec::new(), swi_refill: false })
    }

    #[inline]
    pub fn is_hle(&self) -> bool {
        self.image.is_none()
    }

    /// Word read at `addr` (aligned). `pc_in_bios` = the current opcode fetch came from the BIOS.
    #[inline]
    pub fn read32(&self, addr: u32, pc_in_bios: bool) -> u32 {
        match &self.image {
            Some(img) if pc_in_bios => {
                let i = (addr & 0x3FFC) as usize;
                u32::from_le_bytes([img[i], img[i + 1], img[i + 2], img[i + 3]])
            }
            _ => self.open_bus,
        }
    }

    pub fn take_image(&mut self) -> Option<Box<[u8; BIOS_SIZE]>> {
        self.image.take()
    }

    pub fn set_image(&mut self, image: Option<Box<[u8; BIOS_SIZE]>>) {
        self.image = image;
    }

    pub fn image(&self) -> Option<&[u8]> {
        self.image.as_deref().map(|b| &b[..])
    }
}

/// SWI numbers. Source: GBATEK "BIOS Function Summary".
pub mod swi {
    pub const SOFT_RESET: u8 = 0x00;
    pub const REGISTER_RAM_RESET: u8 = 0x01;
    pub const HALT: u8 = 0x02;
    pub const STOP: u8 = 0x03;
    pub const INTR_WAIT: u8 = 0x04;
    pub const VBLANK_INTR_WAIT: u8 = 0x05;
    pub const DIV: u8 = 0x06;
    pub const DIV_ARM: u8 = 0x07;
    pub const SQRT: u8 = 0x08;
    pub const ARCTAN: u8 = 0x09;
    pub const ARCTAN2: u8 = 0x0A;
    pub const CPU_SET: u8 = 0x0B;
    pub const CPU_FAST_SET: u8 = 0x0C;
    pub const GET_BIOS_CHECKSUM: u8 = 0x0D;
    pub const BG_AFFINE_SET: u8 = 0x0E;
    pub const OBJ_AFFINE_SET: u8 = 0x0F;
    pub const BIT_UNPACK: u8 = 0x10;
    pub const LZ77_UNCOMP_WRAM: u8 = 0x11;
    pub const LZ77_UNCOMP_VRAM: u8 = 0x12;
    pub const HUFF_UNCOMP: u8 = 0x13;
    pub const RL_UNCOMP_WRAM: u8 = 0x14;
    pub const RL_UNCOMP_VRAM: u8 = 0x15;
    pub const DIFF8_UNFILTER_WRAM: u8 = 0x16;
    pub const DIFF8_UNFILTER_VRAM: u8 = 0x17;
    pub const DIFF16_UNFILTER: u8 = 0x18;
    pub const SOUND_BIAS: u8 = 0x19;
    pub const SOUND_DRIVER_INIT: u8 = 0x1A;
    pub const SOUND_DRIVER_MODE: u8 = 0x1B;
    pub const SOUND_DRIVER_MAIN: u8 = 0x1C;
    pub const SOUND_DRIVER_VSYNC: u8 = 0x1D;
    pub const SOUND_CHANNEL_CLEAR: u8 = 0x1E;
    pub const MIDI_KEY2FREQ: u8 = 0x1F;
    pub const SOUND_DRIVER_VSYNC_OFF: u8 = 0x28;
    pub const SOUND_DRIVER_VSYNC_ON: u8 = 0x29;

    pub fn name(n: u8) -> &'static str {
        match n {
            0x00 => "SoftReset",
            0x01 => "RegisterRamReset",
            0x02 => "Halt",
            0x03 => "Stop",
            0x04 => "IntrWait",
            0x05 => "VBlankIntrWait",
            0x06 => "Div",
            0x07 => "DivArm",
            0x08 => "Sqrt",
            0x09 => "ArcTan",
            0x0A => "ArcTan2",
            0x0B => "CpuSet",
            0x0C => "CpuFastSet",
            0x0D => "GetBiosChecksum",
            0x0E => "BgAffineSet",
            0x0F => "ObjAffineSet",
            0x10 => "BitUnPack",
            0x11 => "LZ77UnCompWram",
            0x12 => "LZ77UnCompVram",
            0x13 => "HuffUnComp",
            0x14 => "RLUnCompWram",
            0x15 => "RLUnCompVram",
            0x16 => "Diff8bitUnFilterWram",
            0x17 => "Diff8bitUnFilterVram",
            0x18 => "Diff16bitUnFilter",
            0x19 => "SoundBias",
            0x1A => "SoundDriverInit",
            0x1B => "SoundDriverMode",
            0x1C => "SoundDriverMain",
            0x1D => "SoundDriverVSync",
            0x1E => "SoundChannelClear",
            0x1F => "MidiKey2Freq",
            0x28 => "SoundDriverVSyncOff",
            0x29 => "SoundDriverVSyncOn",
            _ => "?",
        }
    }
}
