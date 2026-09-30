//! Final mix. Source: GBATEK "GBA Sound Control Registers" (SOUNDCNT_L/H, SOUNDBIAS,
//! "Max Output Levels", "Resampling to 32.768kHz / 9bit").
//!
//! Scale: the 10-bit output range is 0..0x3FF. "Each of the two FIFOs can span the FULL output
//! range (+/-200h)" so a FIFO sample (-128..127) is scaled by 4 at 100% and 2 at 50%. "Each of
//! the four PSGs can span one QUARTER of the output range (+/-80h)": a 4-bit level (0..15) at
//! master volume 7 and 100% ratio is scaled by 16 (max 240 ~ 0x100), i.e. level * (vol+1) * 2.
//! The BIAS is added, the sum clipped to 0..0x3FF, reduced to the SOUNDBIAS depth (9/8/7/6 bits)
//! and re-expanded to signed 16 bits so the default bias 0x200 sits at zero.

/// Per-sample inputs to the mixer.
#[derive(Clone, Copy, Debug, Default)]
pub struct MixInput {
    /// Per PSG channel: sum of the 4-bit levels over `ticks` PSG ticks.
    pub psg_sum: [u32; 4],
    /// Number of PSG ticks accumulated (1..=8).
    pub ticks: u32,
    /// SOUNDCNT_L.
    pub cnt_l: u16,
    /// SOUNDCNT_H.
    pub cnt_h: u16,
    /// Current FIFO A/B output samples.
    pub fifo_a: i8,
    pub fifo_b: i8,
    /// SOUNDBIAS.
    pub bias: u16,
    /// SOUNDCNT_X bit 7; when clear both PSG and FIFO output are silent.
    pub master: bool,
}

/// PSG ratio from SOUNDCNT_H bits 0-1 as a left shift: 25% -> 0, 50% -> 1, 100% -> 2.
/// 3 is prohibited; treat as 100%.
#[inline]
fn psg_ratio_shift(cnt_h: u16) -> u32 {
    match cnt_h & 3 {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

/// Mix one stereo output sample. Returns `[left, right]` as signed 16-bit.
pub fn mix(inp: &MixInput) -> [i16; 2] {
    let bias = (inp.bias & 0x3FE) as i32;
    let amp = ((inp.bias >> 14) & 3) as u32;
    let mut out = [0i32; 2];
    if inp.master {
        let ticks = inp.ticks.max(1);
        let shift = psg_ratio_shift(inp.cnt_h);
        // side 0 = left, 1 = right.
        for (side, o) in out.iter_mut().enumerate() {
            let (vol, enables) = if side == 0 {
                (((inp.cnt_l >> 4) & 7) as u32, (inp.cnt_l >> 12) & 0xF)
            } else {
                ((inp.cnt_l & 7) as u32, (inp.cnt_l >> 8) & 0xF)
            };
            let mut sum = 0u32;
            for ch in 0..4 {
                if (enables >> ch) & 1 != 0 {
                    sum += inp.psg_sum[ch];
                }
            }
            // level * (vol+1) * 2 at 100%, halved per ratio step; averaged over the ticks.
            let psg = ((sum * (vol + 1)) << shift) / (2 * ticks);
            let mut v = psg as i32;
            // DMA channels: bit 8/9 = A right/left, 12/13 = B right/left; bit 2/3 = A/B volume.
            let a_on = (inp.cnt_h >> (9 - side as u16)) & 1 != 0;
            let b_on = (inp.cnt_h >> (13 - side as u16)) & 1 != 0;
            if a_on {
                v += (inp.fifo_a as i32) << if inp.cnt_h & 0x04 != 0 { 2 } else { 1 };
            }
            if b_on {
                v += (inp.fifo_b as i32) << if inp.cnt_h & 0x08 != 0 { 2 } else { 1 };
            }
            *o = v;
        }
    }
    let mut res = [0i16; 2];
    for side in 0..2 {
        let v = (out[side] + bias).clamp(0, 0x3FF);
        // 9/8/7/6-bit depth: drop 1+amp bits, then expand to 16 bits centred on 0x200.
        let q = (v >> (1 + amp)) << (7 + amp);
        res[side] = (q - 0x8000) as i16;
    }
    res
}
