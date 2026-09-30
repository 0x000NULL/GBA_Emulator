//! Game pak prefetch buffer (WAITCNT bit 14). Source: GBATEK "GBA GamePak Prefetch" and
//! "GBA System Control" (WAITCNT), checked against the hardware-verified mgba-suite timing tests.
//!
//! While the CPU is not using the game pak bus (internal cycles, accesses to other memory
//! regions) the bus controller keeps reading the opcodes that follow the last ROM opcode fetch,
//! one halfword at a time, into a buffer of eight halfwords. The first halfword costs an S
//! access (the address is sequential to the CPU's fetch); the next one starts the moment the
//! previous one completes. An opcode fetch whose address is the head of the buffer costs one
//! cycle if the halfword has arrived, otherwise the CPU waits for the in-flight halfword.
//! Anything else (branch, first halfword of a 128K block, fetch after the buffer was dropped) is
//! a normal N/S access that restarts the buffer behind it.
//!
//! The buffer is dropped by any data access to the game pak (ROM or SRAM, including DMA), and a
//! data access that arrives while a halfword is in flight waits for it to finish (GBATEK: the
//! bus cannot abort a transfer). A fetch from outside the game pak also drops it.
//!
//! The model is lazy: the buffer is not stepped per cycle. `Prefetch` holds the head address,
//! the number of completed halfwords and the completion time of the in-flight one; `pf_catch_up`
//! folds elapsed time into them at the next access (at most eight iterations).

use super::{Access, Bus};

/// Buffer capacity in halfwords.
const CAPACITY: u8 = 8;
/// The prefetcher cannot cross a 128K block: the first halfword of each block is non-sequential.
const BLOCK_MASK: u32 = 0x1_FFFF;

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "savestate", derive(serde::Serialize, serde::Deserialize))]
pub struct Prefetch {
    /// Mirror of WAITCNT bit 14.
    pub enabled: bool,
    /// The buffer holds data or a halfword fetch is in flight.
    live: bool,
    /// A halfword fetch is in flight, completing at `done`.
    active: bool,
    /// Completed halfwords in the buffer.
    len: u8,
    /// Address of the head (oldest) halfword: the next opcode the CPU is expected to fetch.
    addr: u32,
    /// Absolute cycle at which the in-flight halfword completes.
    done: u64,
}

impl Prefetch {
    /// Drop everything buffered (the in-flight transfer is forgotten; callers that must wait for
    /// it do so before calling this).
    #[inline]
    pub fn invalidate(&mut self) {
        self.live = false;
        self.active = false;
        self.len = 0;
    }

    /// True when a data access has something to wait for or throw away.
    #[inline]
    pub fn is_live(&self) -> bool {
        self.live
    }
}

impl Bus {
    /// Cost of one sequential halfword at `addr` (waitstate + 1).
    #[inline]
    fn pf_s_cost(&self, addr: u32) -> u64 {
        self.rom_s[(((addr >> 24) - 8) / 2) as usize] as u64 + 1
    }

    /// Cost of one halfword access of the given kind at `addr`.
    #[inline]
    fn pf_access_cost(&self, addr: u32, a: Access) -> u64 {
        let region = (((addr >> 24) - 8) / 2) as usize;
        (match a {
            Access::S => self.rom_s[region],
            Access::N => self.rom_n[region],
        }) as u64
            + 1
    }

    /// Start prefetching at `next` once the CPU's own access ends at `t`.
    #[inline]
    fn pf_restart(&mut self, next: u32, t: u64) {
        let in_rom = (0x0800_0000..0x0E00_0000).contains(&next);
        self.pf.live = true;
        self.pf.len = 0;
        self.pf.addr = next;
        if in_rom && next & BLOCK_MASK != 0 {
            self.pf.active = true;
            self.pf.done = t + self.pf_s_cost(next);
        } else {
            self.pf.active = false;
        }
    }

    /// Fold the time up to `now` into the buffer state.
    #[inline]
    fn pf_catch_up(&mut self, now: u64) {
        while self.pf.active && self.pf.done <= now {
            self.pf.len += 1;
            let next = self.pf.addr.wrapping_add(2 * self.pf.len as u32);
            if self.pf.len >= CAPACITY || next & BLOCK_MASK == 0 {
                self.pf.active = false;
            } else {
                self.pf.done += self.pf_s_cost(next);
            }
        }
    }

    /// Cycles the CPU must wait for a transfer that is already under way. A halfword the
    /// prefetcher would only start on this very cycle loses the arbitration to the CPU.
    #[inline]
    fn pf_stall(&self, now: u64) -> u64 {
        if !self.pf.active {
            return 0;
        }
        let next = self.pf.addr.wrapping_add(2 * self.pf.len as u32);
        let start = self.pf.done - self.pf_s_cost(next);
        if start < now {
            self.pf.done - now
        } else {
            0
        }
    }

    /// Try to satisfy `n` consecutive opcode halfwords from the buffer at time `t`, all in one
    /// CPU fetch cycle once the last of them has landed (the buffer hands a whole 32-bit ARM
    /// opcode over at once). Returns the cycles the CPU spends, or `None` (state untouched except
    /// for folding elapsed time) when the buffer cannot supply them all.
    fn pf_take_n(&mut self, addr: u32, n: u32, t: u64) -> Option<u64> {
        if !self.pf.live || self.pf.addr != addr {
            return None;
        }
        if self.pf.active && self.pf.done <= t {
            self.pf_catch_up(t);
        }
        if self.pf.active && self.pf.len as u32 >= n {
            // Hot path: the buffer already holds the whole opcode and the prefetcher is running.
            self.pf.len -= n as u8;
            self.pf.addr = addr.wrapping_add(2 * n);
            return Some(1);
        }
        let was_active = self.pf.active;
        let mut len = self.pf.len as u32;
        let mut need = n;
        let take = len.min(need);
        len -= take;
        need -= take;
        // Halfwords still to come from the transfer chain: each lands one S cost after the last.
        let mut ready = t;
        let mut done = self.pf.done;
        let mut active = was_active;
        let mut next = self.pf.addr.wrapping_add(2 * self.pf.len as u32);
        while need > 0 {
            if !active {
                if self.pf.len == 0 {
                    self.pf.live = false;
                }
                return None;
            }
            ready = done;
            need -= 1;
            next = next.wrapping_add(2);
            if next & BLOCK_MASK == 0 {
                active = false;
            } else {
                done += self.pf_s_cost(next);
            }
        }
        self.pf.addr = addr.wrapping_add(2 * n);
        self.pf.len = len as u8;
        self.pf.done = done;
        self.pf.active = active;
        if !was_active && take > 0 && !active {
            // A slot freed up in a full buffer: the stalled prefetcher resumes behind it.
            let next = self.pf.addr.wrapping_add(2 * len);
            if next & BLOCK_MASK != 0 {
                self.pf.active = true;
                self.pf.done = t + self.pf_s_cost(next);
            }
        }
        if self.pf.len == 0 && !self.pf.active {
            self.pf.live = false;
        }
        Some((ready - t).max(1))
    }

    /// Cycles for an opcode fetch of `width` bytes from game pak ROM with the buffer enabled.
    /// Advances the buffer state; the caller ticks the clock.
    pub(super) fn pf_fetch_cost(&mut self, addr: u32, width: u32, a: Access) -> u32 {
        let start = self.sched.now;
        let mut t = start;
        if width == 4 {
            if let Some(c) = self.pf_take_n(addr, 2, t) {
                return c as u32;
            }
        }
        let mut a = a;
        let mut at = addr;
        let end = addr + width;
        while at < end {
            match self.pf_take_n(at, 1, t) {
                Some(c) => t += c,
                None => {
                    if self.pf.live {
                        // The bus cannot abort a transfer in flight.
                        self.pf_catch_up(t);
                        t += self.pf_stall(t);
                    }
                    t += self.pf_access_cost(at, a);
                    self.pf_restart(at + 2, t);
                }
            }
            a = Access::S;
            at += 2;
        }
        (t - start) as u32
    }

    /// Cycles for a CPU/DMA data access to the game pak (ROM or SRAM) while the buffer is live:
    /// drop the buffer, then the normal access cost plus the stall for the transfer in flight.
    ///
    /// The stall is pinned by the mgba-suite timing rows, not derived from a bus diagram. A CPU
    /// access that finds the halfword in its last cycle waits for it, but one that arrives with two
    /// or more cycles to go cancels the transfer for free (LDM running off OAM into ROM, P.. and
    /// PN. columns). A DMA access (which owns the bus and starts on a cycle boundary) always waits
    /// one cycle for a transfer under way (DMA to ROM, same columns).
    pub(super) fn pf_data_cost(&mut self, addr: u32, width: u32, a: Access) -> u32 {
        let now = self.sched.now;
        self.pf_catch_up(now);
        let remaining = self.pf_stall(now) as u32;
        let stall = if self.in_dma {
            remaining.min(1)
        } else if remaining <= 1 {
            remaining
        } else {
            0
        };
        self.pf.invalidate();
        stall + self.wait(addr, width, a)
    }
}
