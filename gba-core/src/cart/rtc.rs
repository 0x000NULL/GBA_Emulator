//! Seiko S3511 real-time clock on the GPIO port (SCK=bit0, SIO=bit1, CS=bit2).
//! Sources: GBATEK "GBA Cart Real-Time Clock (RTC)" (register set) and "DS Real-Time Clock (RTC)"
//! (serial protocol, docs/gbatek/159-dsrealtimeclockrtc.md).
//!
//! A transfer starts when CS goes high and ends when it goes low. Bits are latched on SCK rising
//! edges, LSB first; the chip drives read data on SIO from each SCK falling edge. The first byte is
//! the command: bits 0-3 the fixed code 0110b, bits 4-6 the register (GBATEK's "Fwd" column),
//! bit 7 read. Games shift the command MSB first (Seiko's advice), e.g. 65h = read datetime; taken
//! LSB first that arrives as A6h, which is the same command in the Fwd column, so one decoder
//! serves both orders.
//!
//! GBA registers (Fwd numbers): 0 force reset (no data), 4 control (1 byte), 2 datetime (7 bytes:
//! year, month, day, day of week, hour, minute, second, BCD), 6 time (3 bytes), 3 force IRQ (no
//! data), 1/5/7 alarm and free registers (read FFh). The hour's AM/PM flag is bit 7 on the GBA.
//!
//! Time model: the host seeds a wall-clock second (`set_clock`) and the clock runs on emulated
//! cycles from there, so headless runs are deterministic. A time the game writes is kept as an
//! offset from that clock, which the cartridge persists in the `.sav` trailer.
//! Not modelled: the /IRQ output (per-minute interrupt).

use crate::timing::CLOCK_HZ;

const CONTROL_24H: u8 = 0x40;
/// Writable control bits: 1 (IRQ duty), 3 (per-minute IRQ), 5 (unknown), 6 (24h). Bit 7, the
/// power-fail flag, always reads 0: the battery never runs dry here.
const CONTROL_MASK: u8 = 0x6A;
/// 2000-01-01, the force-reset time, in days since 1970-01-01.
const DAYS_2000: i64 = 10957;

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Command,
    Write,
    Read,
}

#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Rtc {
    sck: bool,
    cs: bool,
    sio_out: bool,
    phase: Phase,
    shift: u8,
    nbits: u8,
    /// Register of the current transfer (Fwd number).
    reg: u8,
    buf: [u8; 7],
    len: usize,
    /// Bit index into `buf` for reads, byte index for writes.
    pos: usize,
    pub control: u8,
    /// Seconds added to the host clock (set when the game writes a time).
    pub offset: i64,
    /// Added to the computed day of week (0 = Monday), mod 7.
    pub wday_off: u8,
    /// Host wall-clock seconds (local time, as if it were UTC) at `seed_cycle`.
    host_base: i64,
    seed_cycle: u64,
}

impl Rtc {
    pub fn new() -> Self {
        Rtc {
            sck: false,
            cs: false,
            sio_out: false,
            phase: Phase::Idle,
            shift: 0,
            nbits: 0,
            reg: 0,
            buf: [0; 7],
            len: 0,
            pos: 0,
            control: CONTROL_24H,
            offset: 0,
            wday_off: 0,
            host_base: 0,
            seed_cycle: 0,
        }
    }

    /// Seed the host clock: `local_secs` is the wall-clock time in seconds since 1970-01-01 in
    /// local time, valid at scheduler cycle `now`.
    pub fn set_clock(&mut self, local_secs: i64, now: u64) {
        self.host_base = local_secs;
        self.seed_cycle = now;
    }

    fn host_secs(&self, now: u64) -> i64 {
        self.host_base + (now.saturating_sub(self.seed_cycle) / CLOCK_HZ) as i64
    }

    /// The clock's current time in seconds since 1970-01-01.
    pub fn now_secs(&self, now: u64) -> i64 {
        self.host_secs(now) + self.offset
    }

    pub fn sio_out(&self) -> bool {
        self.sio_out
    }

    pub fn write_pins(&mut self, pins: u8, dir: u8, now: u64) {
        let sck = pins & 1 != 0;
        let sio = pins & 2 != 0 && dir & 2 != 0;
        let cs = pins & 4 != 0;
        if !cs {
            // Deselected the chip stops driving SIO; the port then reads it low.
            self.phase = Phase::Idle;
            self.sio_out = false;
        } else if !self.cs {
            self.phase = Phase::Command;
            self.shift = 0;
            self.nbits = 0;
        } else if sck && !self.sck {
            match self.phase {
                Phase::Command | Phase::Write => self.shift_in(sio, now),
                _ => {}
            }
        } else if !sck && self.sck && self.phase == Phase::Read {
            let bit = if self.pos < self.len * 8 { self.buf[self.pos / 8] >> (self.pos % 8) & 1 } else { 1 };
            self.sio_out = bit != 0;
            self.pos += 1;
        }
        self.sck = sck;
        self.cs = cs;
    }

    fn shift_in(&mut self, bit: bool, now: u64) {
        self.shift |= (bit as u8) << self.nbits;
        self.nbits += 1;
        if self.nbits < 8 {
            return;
        }
        let v = self.shift;
        self.shift = 0;
        self.nbits = 0;
        if self.phase == Phase::Command {
            self.command(v, now);
        } else {
            self.buf[self.pos] = v;
            self.pos += 1;
            if self.pos == self.len {
                self.commit(now);
                self.phase = Phase::Idle;
            }
        }
    }

    fn command(&mut self, v: u8, now: u64) {
        if v & 0x0F != 0x06 {
            self.phase = Phase::Idle;
            return;
        }
        self.reg = (v >> 4) & 7;
        let read = v & 0x80 != 0;
        self.len = match self.reg {
            0 | 3 => 0,
            4 => 1,
            2 => 7,
            6 => 3,
            _ => 1,
        };
        self.pos = 0;
        if self.len == 0 {
            // Force reset / force IRQ are strobed by any access.
            if self.reg == 0 {
                self.control = 0;
                self.offset = DAYS_2000 * 86400 - self.host_secs(now);
                self.wday_off = 0;
                self.wday_off = (7 - self.weekday(now)) % 7;
            }
            self.phase = Phase::Idle;
        } else if read {
            self.fill(now);
            self.phase = Phase::Read;
        } else {
            self.phase = Phase::Write;
        }
    }

    fn weekday(&self, now: u64) -> u8 {
        let days = self.now_secs(now).div_euclid(86400);
        // 1970-01-01 was a Thursday: day 3 with Monday = 0.
        ((days + 3).rem_euclid(7) as u8 + self.wday_off) % 7
    }

    fn fill(&mut self, now: u64) {
        let secs = self.now_secs(now);
        let (y, mo, d) = civil_from_days(secs.div_euclid(86400));
        let t = secs.rem_euclid(86400);
        let (h, mi, s) = ((t / 3600) as u8, (t / 60 % 60) as u8, (t % 60) as u8);
        let hour = if self.control & CONTROL_24H != 0 {
            bcd(h) | if h >= 12 { 0x80 } else { 0 }
        } else {
            bcd(h % 12) | if h >= 12 { 0x80 } else { 0 }
        };
        match self.reg {
            4 => self.buf[0] = self.control,
            2 => {
                self.buf = [bcd((y - 2000).rem_euclid(100) as u8), bcd(mo), bcd(d), self.weekday(now), hour, bcd(mi), bcd(s)];
            }
            6 => self.buf[..3].copy_from_slice(&[hour, bcd(mi), bcd(s)]),
            _ => self.buf[0] = 0xFF,
        }
    }

    fn commit(&mut self, now: u64) {
        match self.reg {
            4 => self.control = self.buf[0] & CONTROL_MASK,
            2 | 6 => {
                let cur = self.now_secs(now);
                let (days, wday) = if self.reg == 2 {
                    let y = 2000 + unbcd(self.buf[0]) as i64;
                    let mo = unbcd(self.buf[1] & 0x1F).clamp(1, 12);
                    let d = unbcd(self.buf[2] & 0x3F).max(1);
                    (days_from_civil(y, mo, d), Some(self.buf[3] & 7))
                } else {
                    (cur.div_euclid(86400), None)
                };
                let t = if self.reg == 2 { &self.buf[4..7] } else { &self.buf[..3] };
                let mut h = unbcd(t[0] & 0x3F) as i64;
                if self.control & CONTROL_24H == 0 {
                    h = h % 12 + if t[0] & 0x80 != 0 { 12 } else { 0 };
                }
                let secs = days * 86400 + h * 3600 + unbcd(t[1] & 0x7F) as i64 * 60 + unbcd(t[2] & 0x7F) as i64;
                self.offset += secs - cur;
                if let Some(w) = wday {
                    self.wday_off = 0;
                    self.wday_off = (w % 7 + 7 - self.weekday(now)) % 7;
                }
            }
            _ => {}
        }
    }

    /// Persisted part: offset, weekday offset, control.
    pub fn persist(&self) -> (i64, u8, u8) {
        (self.offset, self.wday_off, self.control)
    }

    pub fn restore(&mut self, offset: i64, wday_off: u8, control: u8) {
        self.offset = offset;
        self.wday_off = wday_off % 7;
        self.control = control & CONTROL_MASK;
    }
}

impl Default for Rtc {
    fn default() -> Self {
        Self::new()
    }
}

fn bcd(v: u8) -> u8 {
    (v / 10) << 4 | v % 10
}

fn unbcd(v: u8) -> u32 {
    (v >> 4) as u32 * 10 + (v & 0xF) as u32
}

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of `days_from_civil`: (year, month, day).
pub fn civil_from_days(z: i64) -> (i64, u8, u8) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}
