//! Debugger hooks: PC breakpoints, memory watchpoints, and the mGBA debug-print registers
//! (0x04FFF600 string buffer, 0x04FFF700 flags, 0x04FFF780 enable) used by mgba-suite/homebrew.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Pc(u32),
    Write { addr: u32, size: u8, value: u32 },
    Read { addr: u32, size: u8 },
}

#[derive(Clone, Debug)]
pub struct DebugHooks {
    /// Master switch; when false the hot paths skip every check.
    pub enabled: bool,
    pub pc_breaks: Vec<u32>,
    /// Inclusive address ranges.
    pub write_watch: Vec<(u32, u32)>,
    pub read_watch: Vec<(u32, u32)>,
    /// Set when a hook fires; `Gba::run_until` stops and clears it.
    pub hit: Option<Hit>,
    mgba_enabled: bool,
    mgba_enable_lo: u8,
    mgba_buf: [u8; 0x100],
    mgba_flags: u16,
    /// Lines printed through the debug registers, in order.
    pub log: Vec<String>,
    /// Log every HLE SWI dispatch (number, name, r0-r3) into `log`.
    pub log_swi: bool,
    /// Bench capture (layer pictures, sound levels, scanline event log). Deliberately not part of
    /// `enabled`: that flag also switches on per-access watch scans.
    pub bench: Option<Box<super::bench::BenchCapture>>,
}

impl Default for DebugHooks {
    fn default() -> Self {
        DebugHooks {
            enabled: false,
            pc_breaks: Vec::new(),
            write_watch: Vec::new(),
            read_watch: Vec::new(),
            hit: None,
            mgba_enabled: false,
            mgba_enable_lo: 0,
            mgba_buf: [0; 0x100],
            mgba_flags: 0,
            log_swi: false,
            log: Vec::new(),
            bench: None,
        }
    }
}

impl DebugHooks {
    pub fn refresh_enabled(&mut self) {
        self.enabled = !self.pc_breaks.is_empty() || !self.write_watch.is_empty() || !self.read_watch.is_empty();
    }

    #[inline]
    pub fn check_pc(&mut self, pc: u32) {
        if self.pc_breaks.contains(&pc) {
            self.hit = Some(Hit::Pc(pc));
        }
    }

    #[inline]
    pub fn check_write(&mut self, addr: u32, size: u8, value: u32) {
        let end = addr + size as u32 - 1;
        if self.write_watch.iter().any(|&(lo, hi)| addr <= hi && end >= lo) {
            self.hit = Some(Hit::Write { addr, size, value });
        }
    }

    #[inline]
    pub fn check_read(&mut self, addr: u32, size: u8) {
        let end = addr + size as u32 - 1;
        if self.read_watch.iter().any(|&(lo, hi)| addr <= hi && end >= lo) {
            self.hit = Some(Hit::Read { addr, size });
        }
    }

    pub fn mgba_buf_write(&mut self, i: usize, v: u8) {
        if i < self.mgba_buf.len() {
            self.mgba_buf[i] = v;
        }
    }

    /// Writing 0xC0DE to 0x04FFF780 enables the interface (reads back 0x1DEA).
    pub fn mgba_enable_write(&mut self, byte: u32, v: u8) {
        // Keep the low byte until the high byte arrives (16-bit writes come as two byte writes).
        if byte == 0 {
            self.mgba_enable_lo = v;
        } else {
            self.mgba_enabled = self.mgba_enable_lo == 0xDE && v == 0xC0;
        }
    }

    /// 0x04FFF700: bits 0-2 level, bit 8 = "flush". Writing the high byte with bit 0 set prints.
    pub fn mgba_flags_write(&mut self, byte: u32, v: u8) {
        if byte == 0 {
            self.mgba_flags = (self.mgba_flags & 0xFF00) | v as u16;
            return;
        }
        self.mgba_flags = (self.mgba_flags & 0x00FF) | ((v as u16) << 8);
        if self.mgba_flags & 0x100 != 0 {
            let len = self.mgba_buf.iter().position(|&b| b == 0).unwrap_or(self.mgba_buf.len());
            let text = String::from_utf8_lossy(&self.mgba_buf[..len]).into_owned();
            let level = self.mgba_flags & 7;
            self.log.push(format!("[{}] {}", level, text));
            self.mgba_buf = [0; 0x100];
            self.mgba_flags &= !0x100;
        }
    }

    pub fn mgba_enabled(&self) -> bool {
        self.mgba_enabled
    }
}
