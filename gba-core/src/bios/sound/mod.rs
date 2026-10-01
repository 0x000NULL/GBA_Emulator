//! HLE of the BIOS sound driver: SWI 1Ah SoundDriverInit, 1Bh SoundDriverMode, 1Ch
//! SoundDriverMain, 1Dh SoundDriverVSync, 1Eh SoundChannelClear, 28h/29h SoundDriverVSyncOff/On
//! and 2Ah SoundGetJumpList (whose entries are emulated in `jumplist.rs`).
//!
//! Source: GBATEK "BIOS Sound Functions" for the contracts, the work-area layout in `area.rs`,
//! and the register sequences the games' own copy of the same library programs (watched on
//! Saibara Rieko no Dendou Mahjong, 2026-10-01: SOUNDCNT_H = A90Eh, SOUNDBIAS high byte 42h,
//! timer 0 stopped / reloaded / started, and the per-period DMA restart "immediate 4 words, stop,
//! FIFO mode" on both channels). The mixer (`mixer.rs`) is a design of our own that follows
//! GBATEK's description of the envelope and wave data.
//!
//! The driver's work area is the game's memory: every field the HLE keeps lives there, so
//! savestates and the game's own reads see one consistent picture.

pub mod area;
pub mod jumplist;
pub mod mixer;
pub mod player;

pub use area::{ID_NUMBER, SOUND_INFO_PTR};

use super::call::{Pending, SavedFrame, SoundMainStage};
use super::HLE_JUMPLIST_BASE;
use area::{info, RateParams};
use crate::bus::{Access, Bus, MemoryBus};
use crate::cpu::regs::Regs;

const REG_SOUNDCNT_H: u32 = 0x0400_0082;
const REG_SOUNDCNT_X: u32 = 0x0400_0084;
const REG_SOUNDBIAS: u32 = 0x0400_0088;
const REG_FIFO_A: u32 = 0x0400_00A0;
const REG_FIFO_B: u32 = 0x0400_00A4;
const REG_DMA1SAD: u32 = 0x0400_00BC;
const REG_DMA1DAD: u32 = 0x0400_00C0;
const REG_DMA1CNT: u32 = 0x0400_00C4;
const REG_DMA1CNT_H: u32 = 0x0400_00C6;
const REG_DMA2SAD: u32 = 0x0400_00C8;
const REG_DMA2DAD: u32 = 0x0400_00CC;
const REG_DMA2CNT: u32 = 0x0400_00D0;
const REG_DMA2CNT_H: u32 = 0x0400_00D2;
const REG_TM0CNT_L: u32 = 0x0400_0100;
const REG_TM0CNT_H: u32 = 0x0400_0102;

/// SOUNDCNT_H: PSG 100%, DMA A and B 100%, A to the right speaker, B to the left, both clocked
/// by timer 0, both FIFOs reset (GBATEK "GBA Sound Control Registers").
const SOUNDCNT_H_INIT: u16 = 0xA90E;
/// SOUNDBIAS: level 200h, 8-bit D/A (the default "9 = 8 bits" of SoundDriverMode bits 20-23).
const SOUNDBIAS_INIT: u16 = 0x4200;
/// DMA1/2CNT as one word: count 4, 32-bit, destination fixed, immediate, enabled: primes the
/// FIFO with the first four words of the buffer.
const DMA_PRIME: u32 = 0x8440_0004;
/// DMA1/2CNT_H: disabled (32-bit).
const DMA_STOP: u16 = 0x0400;
/// DMA1/2CNT_H: enabled, sound-FIFO timing, repeat, 32-bit.
const DMA_FIFO: u16 = 0xB600;
/// TM0CNT_H: run at F/1, no IRQ.
const TM0_RUN: u16 = 0x0080;

/// Lowest address the work area may sit at (EWRAM); anything below is a bad pointer.
const RAM_START: u32 = 0x0200_0000;

impl Bus {
    /// The initialised work area, or None when the pointer at 3007FF0h is unset, points outside
    /// RAM, or the area is not initialised (`ident`). `accept_locked` also accepts the area while
    /// SoundDriverMain owns it (VSync is called from the V-Blank handler, which may interrupt Main).
    pub(crate) fn sound_area(&mut self, accept_locked: bool) -> Option<u32> {
        let area = self.read32(SOUND_INFO_PTR, Access::N) & !3;
        if !(RAM_START..0x0800_0000).contains(&area) {
            return None;
        }
        let ident = self.read32(area + info::IDENT, Access::N);
        if ident == ID_NUMBER || (accept_locked && ident == ID_NUMBER + 1) {
            Some(area)
        } else {
            None
        }
    }

    fn sound_log(&mut self, line: String) {
        if self.debug.log_swi {
            self.debug.log.push(line);
        }
    }

    /// Zero `words` words at `addr`, keeping DMA and timers serviced.
    fn sound_zero(&mut self, addr: u32, words: u32) {
        for i in 0..words {
            self.write32(addr + i * 4, 0, Access::S);
            if i % 64 == 63 {
                self.hle_service();
            }
        }
    }

    /// SWI 1Ah: initialise the work area at `r0` and the sound hardware.
    pub(crate) fn sound_driver_init(&mut self, r0: u32) {
        let area = r0 & !3;
        if !(RAM_START..0x0800_0000).contains(&area) {
            return;
        }
        self.sound_zero(area, info::SIZE / 4);
        self.write32(area + info::IDENT, ID_NUMBER, Access::S);
        self.write8(area + info::MAX_CHANS, 8, Access::S);
        self.write8(area + info::MASTER_VOLUME, 15, Access::S);
        self.write8(area + info::C15, 14, Access::S);
        self.write32(SOUND_INFO_PTR, area, Access::N);
        // Master enable first: the APU drops writes to 60h..81h while it is clear.
        self.write16(REG_SOUNDCNT_X, 0x0080, Access::N);
        self.write16(REG_SOUNDCNT_H, SOUNDCNT_H_INIT, Access::N);
        self.write16(REG_SOUNDBIAS, SOUNDBIAS_INIT, Access::N);
        self.write16(REG_DMA1CNT_H, DMA_STOP, Access::N);
        self.write16(REG_DMA2CNT_H, DMA_STOP, Access::N);
        self.write32(REG_DMA1SAD, area + info::PCM_BUFFER, Access::N);
        self.write32(REG_DMA1DAD, REG_FIFO_A, Access::N);
        self.write32(REG_DMA2SAD, area + info::PCM_BUFFER_LEFT, Access::N);
        self.write32(REG_DMA2DAD, REG_FIFO_B, Access::N);
        let rate = RateParams::for_index(RateParams::DEFAULT_INDEX).unwrap();
        self.sound_set_rate(area, &rate);
        self.sound_log(format!(
            "sound init area={area:08X} hz={} spv={} period={} reload={:04X}",
            rate.hz, rate.samples_per_vblank, rate.dma_period, rate.timer_reload()
        ));
    }

    /// Write the rate-derived fields, clear the PCM buffer, re-arm timer 0 and restart the FIFO
    /// DMAs from the buffer start (SoundDriverInit, SoundDriverMode, SampleFreqSet).
    pub(crate) fn sound_set_rate(&mut self, area: u32, rate: &RateParams) {
        self.write8(area + info::FREQ, rate.index, Access::S);
        self.write8(area + info::DMA_PERIOD, rate.dma_period, Access::S);
        self.write32(area + info::SAMPLES_PER_VBLANK, rate.samples_per_vblank, Access::S);
        self.write32(area + info::PCM_FREQ, rate.hz, Access::S);
        self.write32(area + info::DIV_FREQ, rate.div_freq(), Access::S);
        self.sound_zero(area + info::PCM_BUFFER, 2 * info::PCM_BUF_SIZE / 4);
        self.sound_program_timer(rate.timer_reload());
        self.write8(area + info::DMA_COUNTER, rate.dma_period, Access::S);
        self.sound_restart_dma();
    }

    /// Timer 0 stopped, reloaded, started (the sample clock of both FIFOs).
    fn sound_program_timer(&mut self, reload: u16) {
        self.write16(REG_TM0CNT_H, 0, Access::N);
        self.write16(REG_TM0CNT_L, reload, Access::N);
        self.write16(REG_TM0CNT_H, TM0_RUN, Access::N);
    }

    /// Restart both FIFO DMAs at the buffer start: prime each FIFO with four words by an
    /// immediate transfer, stop, then re-enable in sound-FIFO mode (the library's own sequence;
    /// the source addresses were latched by SoundDriverInit and are reloaded at each enable).
    fn sound_restart_dma(&mut self) {
        self.write32(REG_DMA1CNT, DMA_PRIME, Access::N);
        self.write32(REG_DMA2CNT, DMA_PRIME, Access::N);
        // Immediate transfers start two cycles after the enable and halt the CPU; let them run
        // before the channels are re-armed, as they do between the library's three stores.
        self.idle(4);
        self.hle_service();
        self.write16(REG_DMA1CNT_H, DMA_STOP, Access::N);
        self.write16(REG_DMA2CNT_H, DMA_STOP, Access::N);
        self.write16(REG_DMA1CNT_H, DMA_FIFO, Access::N);
        self.write16(REG_DMA2CNT_H, DMA_FIFO, Access::N);
    }

    /// SWI 1Bh: reverb, channel count, master volume, playback rate, D/A resolution. A zero field
    /// leaves the current value (the titles call it with partial words).
    pub(crate) fn sound_driver_mode(&mut self, mode: u32) {
        let Some(area) = self.sound_area(false) else { return };
        if mode & 0x80 != 0 {
            self.write8(area + info::REVERB, (mode & 0x7F) as u8, Access::S);
        }
        let chans = (mode >> 8) & 0xF;
        if chans != 0 {
            self.write8(area + info::MAX_CHANS, chans.min(info::MAX_CHANNELS) as u8, Access::S);
        }
        let volume = (mode >> 12) & 0xF;
        if volume != 0 {
            self.write8(area + info::MASTER_VOLUME, volume as u8, Access::S);
        }
        let freq = (mode >> 16) & 0xF;
        if freq != 0 {
            if let Some(rate) = RateParams::for_index(freq.min(12) as u8) {
                self.sound_set_rate(area, &rate);
            }
        }
        let dac = (mode >> 20) & 0xF;
        if dac != 0 {
            let resolution = (dac.wrapping_sub(8) & 3) as u16;
            let bias = self.io_read16(REG_SOUNDBIAS) & 0x3FFF;
            self.write16(REG_SOUNDBIAS, bias | (resolution << 14), Access::N);
        }
        self.sound_log(format!("sound mode {mode:08X} reverb={} chans={chans} vol={volume} freq={freq} dac={dac}", mode & 0xFF));
    }

    /// SWI 1Ch: lock the area, run the game's music player (`func(intp)`) and CGB routine, mix one
    /// frame of PCM into the ring, unlock. The two calls are guest code, so the routine is staged
    /// through the continuation mechanism (`call.rs`).
    pub(crate) fn sound_driver_main(&mut self, regs: &mut Regs) {
        let Some(area) = self.sound_area(false) else { return };
        self.write32(area + info::IDENT, ID_NUMBER + 1, Access::S);
        let c15 = self.read8(area + info::C15, Access::N);
        self.write8(area + info::C15, if c15 == 0 { 14 } else { c15 - 1 }, Access::S);
        let saved = SavedFrame::from_swi(regs);
        self.sound_main_continue(regs, saved, area, SoundMainStage::Start, false);
    }

    /// A callback of SoundDriverMain returned.
    pub(crate) fn sound_main_resume(&mut self, regs: &mut Regs, saved: SavedFrame, area: u32, stage: SoundMainStage) {
        self.sound_main_continue(regs, saved, area, stage, true);
    }

    fn sound_main_continue(&mut self, regs: &mut Regs, saved: SavedFrame, area: u32, stage: SoundMainStage, resumed: bool) {
        let mut stage = stage;
        loop {
            match stage {
                SoundMainStage::Start => {
                    let func = self.read32(area + info::FUNC, Access::N);
                    if func != 0 {
                        let intp = self.read32(area + info::INTP, Access::N);
                        let pending = Pending::SoundMain { area, stage: SoundMainStage::AfterFunc };
                        self.hle_call_guest(regs, saved, func, &[intp], pending);
                        return;
                    }
                    stage = SoundMainStage::AfterFunc;
                }
                SoundMainStage::AfterFunc => {
                    let cgb = self.read32(area + info::CGB_SOUND, Access::N);
                    if cgb != 0 {
                        let pending = Pending::SoundMain { area, stage: SoundMainStage::AfterCgb };
                        self.hle_call_guest(regs, saved, cgb, &[], pending);
                        return;
                    }
                    stage = SoundMainStage::AfterCgb;
                }
                SoundMainStage::AfterCgb => {
                    self.sound_mix_frame(area);
                    self.write32(area + info::IDENT, ID_NUMBER, Access::S);
                    if resumed {
                        self.hle_call_finish(regs, &saved, true);
                    }
                    return;
                }
            }
        }
    }

    /// SWI 1Dh: count the V-Blank; when the period has elapsed, restart the FIFO DMAs at the
    /// buffer start. The counter rule (restart when the decremented count reaches zero or wraps)
    /// is the one the library's own VSync shows (watched: 9,8,...,1,0 -> restart and reload).
    pub(crate) fn sound_driver_vsync(&mut self) {
        let Some(area) = self.sound_area(true) else { return };
        let cnt = self.read8(area + info::DMA_COUNTER, Access::N).wrapping_sub(1);
        self.write8(area + info::DMA_COUNTER, cnt, Access::S);
        if (cnt as i8) <= 0 {
            let period = self.read8(area + info::DMA_PERIOD, Access::N).max(1);
            self.write8(area + info::DMA_COUNTER, period, Access::S);
            self.sound_restart_dma();
        }
    }

    /// SWI 1Eh: "clears all direct sound channels and stops the sound": every channel structure
    /// and the PCM buffer are zeroed. A no-op before SoundDriverInit (Atari Anniversary Advance
    /// calls it first).
    pub(crate) fn sound_channel_clear(&mut self) {
        let Some(area) = self.sound_area(true) else { return };
        self.sound_zero(area + info::CHANS, info::MAX_CHANNELS * info::CHAN_SIZE / 4);
        self.sound_zero(area + info::PCM_BUFFER, 2 * info::PCM_BUF_SIZE / 4);
    }

    /// SWI 28h: stop the sound DMA (the game cannot call VSync every frame for a while). The area
    /// is marked so that SoundDriverMain and VSync leave it alone until VSyncOn.
    pub(crate) fn sound_vsync_off(&mut self) {
        let Some(area) = self.sound_area(true) else { return };
        self.write32(area + info::IDENT, ID_NUMBER + 10, Access::S);
        self.write16(REG_DMA1CNT_H, DMA_STOP, Access::N);
        self.write16(REG_DMA2CNT_H, DMA_STOP, Access::N);
    }

    /// SWI 29h: resume after VSyncOff; the next SoundDriverVSync (GBATEK: within 2/60 s) restarts
    /// the DMA from the buffer start.
    pub(crate) fn sound_vsync_on(&mut self) {
        let area = self.read32(SOUND_INFO_PTR, Access::N) & !3;
        if !(RAM_START..0x0800_0000).contains(&area) || self.read32(area + info::IDENT, Access::N) != ID_NUMBER + 10 {
            return;
        }
        self.write32(area + info::IDENT, ID_NUMBER, Access::S);
        self.write8(area + info::DMA_COUNTER, 1, Access::S);
    }

    /// SWI 2Ah: the 36 jump-list entries, each a distinct BIOS-region address the CPU traps on
    /// (`jumplist.rs`). Exactly 36 words: the library allocates 90h bytes and keeps other data
    /// right after the table.
    pub(crate) fn sound_get_jump_list(&mut self, dst: u32) {
        let dst = dst & !3;
        for i in 0..super::HLE_JUMPLIST_SLOTS {
            self.write32(dst.wrapping_add(i * 4), HLE_JUMPLIST_BASE + i * 4, Access::S);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::area::{chan, status};
    use super::*;
    use crate::bios::swi;
    use crate::bios::test_util::{bus, swi};
    use crate::cpu::regs::MODE_SYS;

    const AREA: u32 = 0x0300_2810;

    fn init(b: &mut Bus) {
        swi(b, swi::SOUND_DRIVER_INIT, [AREA, 0, 0, 0]);
    }

    #[test]
    fn init_writes_the_area_and_registers() {
        let mut b = bus();
        // Garbage in the area beforehand must be cleared.
        b.write32(AREA + 0x200, 0xDEAD_BEEF, Access::N);
        init(&mut b);
        assert_eq!(b.peek32(SOUND_INFO_PTR), AREA);
        assert_eq!(b.peek32(AREA), ID_NUMBER);
        assert_eq!(b.peek32(AREA + 0x200), 0);
        assert_eq!(b.peek8(AREA + info::DMA_COUNTER), 7);
        assert_eq!(b.peek8(AREA + info::MAX_CHANS), 8);
        assert_eq!(b.peek8(AREA + info::MASTER_VOLUME), 15);
        assert_eq!(b.peek8(AREA + info::FREQ), 4);
        assert_eq!(b.peek8(AREA + info::C15), 14);
        assert_eq!(b.peek8(AREA + info::DMA_PERIOD), 7);
        assert_eq!(b.peek32(AREA + info::SAMPLES_PER_VBLANK), 224);
        assert_eq!(b.peek32(AREA + info::PCM_FREQ), 13379);
        assert_eq!(b.peek32(AREA + info::DIV_FREQ), 627);
        assert_eq!(b.peek32(AREA + info::FUNC), 0);
        assert_eq!(b.io_read16(REG_SOUNDCNT_X) & 0x80, 0x80);
        assert_eq!(b.io_read16(REG_SOUNDCNT_H) & 0x770F, SOUNDCNT_H_INIT & 0x770F, "routing and volumes (reset bits read as 0)");
        assert_eq!(b.io_read16(REG_SOUNDBIAS), SOUNDBIAS_INIT);
        assert_eq!(b.timers.t[0].reload, 0xFB1A);
        assert!(b.timers.t[0].enabled());
        assert_eq!(b.dma.ch[1].sad, AREA + info::PCM_BUFFER);
        assert_eq!(b.dma.ch[1].dad, REG_FIFO_A);
        assert_eq!(b.dma.ch[2].sad, AREA + info::PCM_BUFFER_LEFT);
        assert_eq!(b.dma.ch[2].dad, REG_FIFO_B);
        for ch in 1..=2 {
            assert!(b.dma.ch[ch].enabled(), "DMA{ch} running");
            assert_eq!(b.dma.ch[ch].timing(), 3, "DMA{ch} in sound-FIFO mode");
            assert!(b.dma.is_fifo(ch));
        }
        assert_eq!(b.apu.fifo_a.len(), 16, "FIFO A primed with four words");
        assert_eq!(b.apu.fifo_b.len(), 16, "FIFO B primed with four words");
        assert!(b.bios.unimplemented.is_empty());
    }

    #[test]
    fn init_rejects_a_pointer_outside_ram() {
        let mut b = bus();
        swi(&mut b, swi::SOUND_DRIVER_INIT, [0x0800_0000, 0, 0, 0]);
        assert_eq!(b.peek32(SOUND_INFO_PTR), 0);
        assert!(b.sound_area(true).is_none());
    }

    #[test]
    fn mode_zero_fields_keep_values() {
        let mut b = bus();
        init(&mut b);
        // Lizzie McGuire's first call: 1 channel, rate 12, everything else untouched.
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x000C_0100, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::MAX_CHANS), 1);
        assert_eq!(b.peek8(AREA + info::MASTER_VOLUME), 15);
        assert_eq!(b.peek8(AREA + info::FREQ), 12);
        assert_eq!(b.peek32(AREA + info::SAMPLES_PER_VBLANK), 704);
        assert_eq!(b.peek8(AREA + info::DMA_PERIOD), 2);
        assert_eq!(b.peek8(AREA + info::DMA_COUNTER), 2);
        assert_eq!(b.timers.t[0].reload, 0xFE71);
        // Second call: 4 channels, rate 4, 9-bit D/A; the volume is still untouched.
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0084_0400, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::MAX_CHANS), 4);
        assert_eq!(b.peek8(AREA + info::MASTER_VOLUME), 15);
        assert_eq!(b.peek8(AREA + info::FREQ), 4);
        assert_eq!(b.timers.t[0].reload, 0xFB1A);
        assert_eq!(b.io_read16(REG_SOUNDBIAS), 0x0200, "dac 8 = 9 bits");
        // Reverb needs bit 7.
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0000_0040, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::REVERB), 0);
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0000_00C0, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::REVERB), 0x40);
        // Pocket Professor: 8 channels, volume 15, rate 7, 8-bit.
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0097_F800, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::MAX_CHANS), 8);
        assert_eq!(b.peek32(AREA + info::PCM_FREQ), 21024);
        assert_eq!(b.io_read16(REG_SOUNDBIAS), 0x4200);
    }

    #[test]
    fn mode_without_init_is_ignored() {
        let mut b = bus();
        swi(&mut b, swi::SOUND_DRIVER_MODE, [0x0097_F800, 0, 0, 0]);
        assert_eq!(b.timers.t[0].reload, 0);
    }

    #[test]
    fn vsync_counts_down_and_restarts_the_dma_every_period() {
        let mut b = bus();
        init(&mut b);
        // Let the FIFO DMA run a while so the source pointer has advanced.
        for _ in 0..4 {
            b.idle(20_000);
            b.hle_service();
        }
        assert!(b.dma.ch[1].src > AREA + info::PCM_BUFFER + 16, "DMA1 streamed past the primed words");
        let mut counts = Vec::new();
        for _ in 0..7 {
            swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
            counts.push(b.peek8(AREA + info::DMA_COUNTER));
        }
        assert_eq!(counts, vec![6, 5, 4, 3, 2, 1, 7]);
        // The FIFO-mode re-enable latches the source again, so the stream restarts at the
        // buffer start (the priming transfer fed the FIFO from the same words).
        assert_eq!(b.dma.ch[1].src, AREA + info::PCM_BUFFER, "restarted at the buffer start");
        assert_eq!(b.dma.ch[2].src, AREA + info::PCM_BUFFER_LEFT);
        assert!(b.dma.is_fifo(1) && b.dma.is_fifo(2));
        // VSync also serves while Main holds the lock.
        b.write32(AREA, ID_NUMBER + 1, Access::N);
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::DMA_COUNTER), 6);
    }

    #[test]
    fn channel_clear_zeroes_channels_and_buffer_and_is_a_no_op_before_init() {
        let mut b = bus();
        b.write32(SOUND_INFO_PTR, AREA, Access::N);
        b.write8(AREA + info::CHANS, 0x80, Access::N);
        swi(&mut b, swi::SOUND_CHANNEL_CLEAR, [0, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::CHANS), 0x80, "uninitialised area left alone");
        init(&mut b);
        b.write8(AREA + info::CHANS + 5 * info::CHAN_SIZE + chan::STATUS, status::START, Access::N);
        b.write32(AREA + info::PCM_BUFFER + 100, 0x1234_5678, Access::N);
        b.write32(AREA + info::PCM_BUFFER_LEFT + 100, 0x1234_5678, Access::N);
        swi(&mut b, swi::SOUND_CHANNEL_CLEAR, [0, 0, 0, 0]);
        assert_eq!(b.peek8(AREA + info::CHANS + 5 * info::CHAN_SIZE), 0);
        assert_eq!(b.peek32(AREA + info::PCM_BUFFER + 100), 0);
        assert_eq!(b.peek32(AREA + info::PCM_BUFFER_LEFT + 100), 0);
        assert_eq!(b.peek32(AREA), ID_NUMBER, "ident kept");
        assert_eq!(b.peek32(AREA + info::PCM_FREQ), 13379, "header kept");
    }

    #[test]
    fn vsync_off_stops_the_dma_until_vsync_on_and_the_next_vsync() {
        let mut b = bus();
        init(&mut b);
        swi(&mut b, swi::SOUND_DRIVER_VSYNC_OFF, [0, 0, 0, 0]);
        assert!(!b.dma.ch[1].enabled() && !b.dma.ch[2].enabled());
        assert_eq!(b.peek32(AREA), ID_NUMBER + 10);
        // Main and VSync do nothing while off.
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        assert!(!b.dma.ch[1].enabled());
        let regs = swi(&mut b, swi::SOUND_DRIVER_MAIN, [0, 0, 0, 0]);
        assert_eq!(regs.r[15], 0x0800_0004);
        assert!(b.bios.calls.is_empty());
        swi(&mut b, swi::SOUND_DRIVER_VSYNC_ON, [0, 0, 0, 0]);
        assert_eq!(b.peek32(AREA), ID_NUMBER);
        swi(&mut b, swi::SOUND_DRIVER_VSYNC, [0, 0, 0, 0]);
        assert!(b.dma.ch[1].enabled() && b.dma.ch[2].enabled());
        assert_eq!(b.peek8(AREA + info::DMA_COUNTER), 7);
    }

    #[test]
    fn get_jump_list_fills_exactly_36_trap_addresses() {
        let mut b = bus();
        b.write32(0x0300_6C40 + 0x90, 0x1111_1111, Access::N);
        swi(&mut b, swi::SOUND_GET_JUMP_LIST, [0x0300_6C40, 0, 0, 0]);
        for i in 0..36u32 {
            assert_eq!(b.peek32(0x0300_6C40 + i * 4), HLE_JUMPLIST_BASE + i * 4, "slot {i}");
        }
        assert_eq!(b.peek32(0x0300_6C40 + 0x90), 0x1111_1111, "the word after the table is the game's");
        assert!(b.bios.unimplemented.is_empty());
    }

    #[test]
    fn main_calls_func_then_cgbsound_then_mixes_and_rejects_reentry() {
        let mut b = bus();
        init(&mut b);
        b.write32(AREA + info::FUNC, 0x0803_B4A9, Access::N);
        b.write32(AREA + info::INTP, 0x0300_6E50, Access::N);
        b.write32(AREA + info::CGB_SOUND, 0x0803_C594, Access::N);
        let mut regs = Regs::new();
        regs.set_cpsr(MODE_SYS);
        regs.r[1] = 0x1111;
        regs.r[14] = 0x0800_0F01;
        regs.r[15] = 0x0800_0004;
        assert!(b.hle_swi(swi::SOUND_DRIVER_MAIN, &mut regs));
        assert_eq!(regs.r[15], 0x0803_B4A8);
        assert!(regs.thumb());
        assert_eq!(regs.r[0], 0x0300_6E50);
        assert_eq!(regs.r[14], crate::bios::HLE_CALL_RETURN_ADDR);
        assert_eq!(b.peek32(AREA), ID_NUMBER + 1, "locked while the player runs");
        assert_eq!(b.peek8(AREA + info::C15), 13);
        // A nested SoundDriverMain (from an IRQ handler) is refused while locked.
        let mut nested = Regs::new();
        nested.set_cpsr(MODE_SYS);
        nested.r[15] = 0x0300_0104;
        assert!(b.hle_swi(swi::SOUND_DRIVER_MAIN, &mut nested));
        assert_eq!(nested.r[15], 0x0300_0104);
        assert_eq!(b.bios.calls.len(), 1);
        // The player returns: CgbSound is called next, in ARM state.
        regs.r[15] = crate::bios::HLE_CALL_RETURN_ADDR;
        assert!(b.hle_vector(&mut regs));
        assert_eq!(regs.r[15], 0x0803_C594);
        assert!(!regs.thumb());
        assert_eq!(regs.r[14], crate::bios::HLE_CALL_RETURN_ADDR);
        assert_eq!(b.peek32(AREA), ID_NUMBER + 1);
        // CgbSound returns: the frame is mixed and the caller resumes.
        regs.r[15] = crate::bios::HLE_CALL_RETURN_ADDR;
        assert!(b.hle_vector(&mut regs));
        assert!(b.bios.calls.is_empty());
        assert_eq!(b.peek32(AREA), ID_NUMBER, "unlocked");
        assert_eq!(regs.r[15], 0x0800_0004);
        assert!(!regs.thumb());
        assert_eq!(regs.r[1], 0x1111);
        assert_eq!(regs.r[14], 0x0800_0F01);
    }

    #[test]
    fn main_without_callbacks_returns_directly() {
        let mut b = bus();
        init(&mut b);
        let regs = swi(&mut b, swi::SOUND_DRIVER_MAIN, [7, 8, 9, 10]);
        assert_eq!(regs.r[15], 0x0800_0004);
        assert_eq!(regs.r[..4], [7, 8, 9, 10]);
        assert!(b.bios.calls.is_empty());
        assert_eq!(b.peek32(AREA), ID_NUMBER);
    }
}
