//! Audio output: lock-free SPSC ring buffer, linear resampler, cpal (WASAPI) stream.
//!
//! The emulation thread resamples the core's stereo i16 samples to the device rate and pushes
//! them into the ring; the cpal callback pops them. When the ring is empty the callback outputs
//! silence and counts an underrun (once per callback), except before the buffer has been primed
//! for the first time.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------
// Ring buffer
// ---------------------------------------------------------------------------------------------

/// Output buffering, in seconds of device-rate output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Buffering {
    /// Ring buffer capacity.
    pub ring_secs: f64,
    /// The fill level audio-clocked pacing steers towards.
    pub target_secs: f64,
}

/// Single-producer single-consumer ring of stereo i16 frames, packed into `AtomicU32` slots so no
/// `unsafe` is needed. `head`/`tail` are free-running counters; the slot index is `pos % cap`.
pub struct RingBuffer {
    slots: Box<[AtomicU32]>,
    cap: usize,
    /// Next frame to read (consumer-owned).
    head: AtomicUsize,
    /// Next frame to write (producer-owned).
    tail: AtomicUsize,
}

#[inline]
fn pack(s: [i16; 2]) -> u32 {
    (s[0] as u16 as u32) | ((s[1] as u16 as u32) << 16)
}

#[inline]
fn unpack(v: u32) -> [i16; 2] {
    [(v & 0xFFFF) as u16 as i16, (v >> 16) as u16 as i16]
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        let slots = (0..cap).map(|_| AtomicU32::new(0)).collect::<Vec<_>>().into_boxed_slice();
        RingBuffer { slots, cap, head: AtomicUsize::new(0), tail: AtomicUsize::new(0) }
    }

    /// Frames currently buffered.
    pub fn len(&self) -> usize {
        let t = self.tail.load(Ordering::Acquire);
        let h = self.head.load(Ordering::Acquire);
        t.wrapping_sub(h).min(self.cap)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Producer side. Returns false (and drops the frame) when full.
    pub fn push(&self, s: [i16; 2]) -> bool {
        let t = self.tail.load(Ordering::Relaxed);
        let h = self.head.load(Ordering::Acquire);
        if t.wrapping_sub(h) >= self.cap {
            return false;
        }
        self.slots[t % self.cap].store(pack(s), Ordering::Relaxed);
        self.tail.store(t.wrapping_add(1), Ordering::Release);
        true
    }

    /// Producer side. Returns the number of frames that did not fit.
    pub fn push_slice(&self, frames: &[[i16; 2]]) -> usize {
        let mut dropped = 0;
        for &f in frames {
            if !self.push(f) {
                dropped += 1;
            }
        }
        dropped
    }

    /// Consumer side.
    pub fn pop(&self) -> Option<[i16; 2]> {
        let h = self.head.load(Ordering::Relaxed);
        let t = self.tail.load(Ordering::Acquire);
        if h == t {
            return None;
        }
        let v = self.slots[h % self.cap].load(Ordering::Relaxed);
        self.head.store(h.wrapping_add(1), Ordering::Release);
        Some(unpack(v))
    }

    /// Consumer side: discard everything buffered.
    pub fn clear(&self) {
        let t = self.tail.load(Ordering::Acquire);
        self.head.store(t, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------------------------
// Resampler
// ---------------------------------------------------------------------------------------------

/// Linear-interpolation resampler for stereo i16 frames. Stateful across calls so streams may be
/// fed in arbitrary chunks.
pub struct Resampler {
    /// Source frames advanced per output frame.
    step: f64,
    /// Position on the timeline where 0.0 is `prev` and 1.0 is the first frame of the next input.
    pos: f64,
    prev: [i16; 2],
}

impl Resampler {
    pub fn new(src_rate: f64, dst_rate: f64) -> Self {
        let mut r = Resampler { step: 1.0, pos: 1.0, prev: [0, 0] };
        r.set_rates(src_rate, dst_rate);
        r
    }

    pub fn set_rates(&mut self, src_rate: f64, dst_rate: f64) {
        assert!(src_rate > 0.0 && dst_rate > 0.0, "sample rates must be positive");
        self.step = src_rate / dst_rate;
    }

    /// Resample `input`, appending output frames to `out`. Every input frame is consumed.
    pub fn process(&mut self, input: &[[i16; 2]], out: &mut Vec<[i16; 2]>) {
        if input.is_empty() {
            return;
        }
        let n = input.len() as f64;
        while self.pos < n {
            let i = self.pos as usize; // floor, pos >= 0
            let frac = self.pos - i as f64;
            let a = if i == 0 { self.prev } else { input[i - 1] };
            let b = input[i];
            out.push([lerp(a[0], b[0], frac), lerp(a[1], b[1], frac)]);
            self.pos += self.step;
        }
        self.pos -= n;
        self.prev = input[input.len() - 1];
    }
}

#[inline]
fn lerp(a: i16, b: i16, t: f64) -> i16 {
    (a as f64 + (b as f64 - a as f64) * t).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

// ---------------------------------------------------------------------------------------------
// Device output
// ---------------------------------------------------------------------------------------------

struct Shared {
    ring: RingBuffer,
    underruns: AtomicU64,
    /// Set once the ring reached the target fill; before that the callback plays silence quietly.
    primed: AtomicBool,
    /// Output gain as f32 bits, applied in the callback so a volume change is heard at once.
    gain: AtomicU32,
}

impl Shared {
    fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }
}

/// Volume 0..=10 to a gain: a square law reads as evenly spaced loudness steps.
pub fn volume_gain(volume: u8) -> f32 {
    let v = volume.min(10) as f32 / 10.0;
    v * v
}

pub struct AudioOut {
    _stream: cpal::Stream,
    shared: Arc<Shared>,
    resampler: Resampler,
    scratch: Vec<[i16; 2]>,
    src_rate: f64,
    device_rate: u32,
    target_fill: usize,
    dropped: u64,
    muted: bool,
    device_name: String,
}

impl AudioOut {
    /// Names of the output devices the host offers.
    pub fn list_devices() -> Vec<String> {
        let host = cpal::default_host();
        match host.output_devices() {
            Ok(it) => it.filter_map(|d| d.name().ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Open the named output device (`None` or an unknown name = the default device), with a ring
    /// of `buffering.ring_secs` that audio-clocked pacing keeps near `buffering.target_secs`.
    pub fn open_on(src_rate: f64, muted: bool, name: Option<&str>, buffering: Buffering) -> Result<AudioOut> {
        let host = cpal::default_host();
        let named = name.and_then(|n| host.output_devices().ok()?.find(|d| d.name().map(|dn| dn == n).unwrap_or(false)));
        if name.is_some() && named.is_none() {
            eprintln!("warning: audio device {:?} not found; using the default", name.unwrap_or(""));
        }
        let device = match named {
            Some(d) => d,
            None => host.default_output_device().ok_or_else(|| anyhow!("no default output device"))?,
        };
        let supported = device.default_output_config().context("querying default output config")?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let device_rate = config.sample_rate.0;
        let channels = config.channels;
        if channels == 0 {
            return Err(anyhow!("output device reports zero channels"));
        }

        let cap = (device_rate as f64 * buffering.ring_secs) as usize;
        let target_fill = (device_rate as f64 * buffering.target_secs) as usize;
        let shared = Arc::new(Shared {
            ring: RingBuffer::new(cap),
            underruns: AtomicU64::new(0),
            primed: AtomicBool::new(false),
            gain: AtomicU32::new(1.0f32.to_bits()),
        });

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(&device, &config, shared.clone()),
            SampleFormat::I16 => build_stream::<i16>(&device, &config, shared.clone()),
            SampleFormat::U16 => build_stream::<u16>(&device, &config, shared.clone()),
            SampleFormat::I32 => build_stream::<i32>(&device, &config, shared.clone()),
            SampleFormat::F64 => build_stream::<f64>(&device, &config, shared.clone()),
            other => Err(anyhow!("unsupported device sample format {other:?}")),
        }?;
        stream.play().context("starting audio stream")?;

        let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());
        println!(
            "audio: {} @ {} Hz, {} ch, {:?}, ring {} frames, target {} frames",
            device_name,
            device_rate,
            channels,
            sample_format,
            cap,
            target_fill
        );

        Ok(AudioOut {
            _stream: stream,
            shared,
            resampler: Resampler::new(src_rate, device_rate as f64),
            scratch: Vec::with_capacity(4096),
            src_rate,
            device_rate,
            target_fill,
            dropped: 0,
            muted,
            device_name,
        })
    }

    /// Set the output gain (1.0 = unity); takes effect on the next callback.
    pub fn set_gain(&self, gain: f32) {
        self.shared.gain.store(gain.clamp(0.0, 4.0).to_bits(), Ordering::Relaxed);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn gain(&self) -> f32 {
        self.shared.gain()
    }

    /// Mute at the source (zeros are queued) so the pacing clock keeps running.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Update the core's sample rate (SOUNDBIAS can change it at runtime).
    pub fn set_src_rate(&mut self, src_rate: f64) {
        if (src_rate - self.src_rate).abs() > 0.5 {
            self.src_rate = src_rate;
            self.resampler.set_rates(src_rate, self.device_rate as f64);
        }
    }

    /// Resample and enqueue core samples. Frames that do not fit are dropped and counted.
    pub fn push(&mut self, samples: &[[i16; 2]]) {
        self.scratch.clear();
        self.resampler.process(samples, &mut self.scratch);
        if self.muted {
            for s in &mut self.scratch {
                *s = [0, 0];
            }
        }
        self.dropped += self.shared.ring.push_slice(&self.scratch) as u64;
        if !self.shared.primed.load(Ordering::Relaxed) && self.shared.ring.len() >= self.target_fill {
            self.shared.primed.store(true, Ordering::Release);
        }
    }

    /// Frames waiting in the ring.
    pub fn fill(&self) -> usize {
        self.shared.ring.len()
    }

    /// Fill level the pacer aims for.
    pub fn target_fill(&self) -> usize {
        self.target_fill
    }

    pub fn underruns(&self) -> u64 {
        self.shared.underruns.load(Ordering::Relaxed)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Discard buffered audio (fast-forward) and re-arm priming so the restart is silent.
    pub fn flush(&mut self) {
        self.shared.ring.clear();
        self.shared.primed.store(false, Ordering::Release);
    }
}

fn build_stream<T>(device: &cpal::Device, config: &StreamConfig, shared: Arc<Shared>) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let err_fn = |e| eprintln!("audio stream error: {e}");
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| fill_output(data, channels, &shared),
            err_fn,
            None,
        )
        .context("building output stream")?;
    Ok(stream)
}

/// Callback body: interleave ring frames into `data`; silence (and one underrun) on shortfall.
fn fill_output<T: SizedSample + FromSample<f32>>(data: &mut [T], channels: usize, shared: &Shared) {
    let silence = T::from_sample(0.0f32);
    if !shared.primed.load(Ordering::Acquire) {
        data.fill(silence);
        return;
    }
    let mut short = false;
    let gain = shared.gain();
    for frame in data.chunks_mut(channels) {
        match shared.ring.pop() {
            Some([l, r]) => {
                let l = T::from_sample((l as f32 / 32768.0 * gain).clamp(-1.0, 1.0));
                let r = T::from_sample((r as f32 / 32768.0 * gain).clamp(-1.0, 1.0));
                for (i, s) in frame.iter_mut().enumerate() {
                    *s = match i {
                        0 => l,
                        1 => r,
                        _ => silence,
                    };
                }
            }
            None => {
                short = true;
                frame.fill(silence);
            }
        }
    }
    if short {
        shared.underruns.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_fifo_order_and_capacity() {
        let r = RingBuffer::new(4);
        assert!(r.is_empty());
        for i in 0..4 {
            assert!(r.push([i, -i]));
        }
        assert_eq!(r.len(), 4);
        assert!(!r.push([99, 99]), "full ring rejects");
        assert_eq!(r.pop(), Some([0, 0]));
        assert_eq!(r.pop(), Some([1, -1]));
        assert!(r.push([5, 5]));
        assert_eq!(r.pop(), Some([2, -2]));
        assert_eq!(r.pop(), Some([3, -3]));
        assert_eq!(r.pop(), Some([5, 5]));
        assert_eq!(r.pop(), None);
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn ring_wraps_many_times() {
        let r = RingBuffer::new(3);
        for i in 0..1000i16 {
            assert!(r.push([i, i.wrapping_neg()]));
            assert_eq!(r.pop(), Some([i, i.wrapping_neg()]));
        }
        assert_eq!(r.push_slice(&[[1, 1], [2, 2], [3, 3], [4, 4]]), 1);
        assert_eq!(r.len(), 3);
        r.clear();
        assert!(r.is_empty());
    }

    #[test]
    fn ring_preserves_negative_samples() {
        let r = RingBuffer::new(2);
        r.push([i16::MIN, i16::MAX]);
        r.push([-1, 1]);
        assert_eq!(r.pop(), Some([i16::MIN, i16::MAX]));
        assert_eq!(r.pop(), Some([-1, 1]));
    }

    #[test]
    fn ring_cross_thread() {
        let r = Arc::new(RingBuffer::new(64));
        let p = r.clone();
        let n = 10_000i32;
        let producer = std::thread::spawn(move || {
            for i in 0..n {
                let s = [(i & 0x7FFF) as i16, (i >> 15) as i16];
                while !p.push(s) {
                    std::hint::spin_loop();
                }
            }
        });
        let mut expect = 0i32;
        while expect < n {
            if let Some(s) = r.pop() {
                assert_eq!(s, [(expect & 0x7FFF) as i16, (expect >> 15) as i16]);
                expect += 1;
            }
        }
        producer.join().unwrap();
    }

    #[test]
    fn resampler_identity_passes_samples_through() {
        let mut rs = Resampler::new(32768.0, 32768.0);
        let mut out = Vec::new();
        rs.process(&[[1, -1], [2, -2], [3, -3]], &mut out);
        rs.process(&[[4, -4], [5, -5]], &mut out);
        rs.process(&[[6, -6]], &mut out);
        // One frame of latency, then every input frame exactly once.
        assert_eq!(out, vec![[1, -1], [2, -2], [3, -3], [4, -4], [5, -5]]);
    }

    #[test]
    fn resampler_upsample_interpolates_midpoints() {
        let mut rs = Resampler::new(1000.0, 2000.0);
        let mut out = Vec::new();
        rs.process(&[[0, 0], [100, -100], [200, -200]], &mut out);
        assert_eq!(out, vec![[0, 0], [50, -50], [100, -100], [150, -150]]);
        // Continues seamlessly across chunk boundaries.
        rs.process(&[[300, -300]], &mut out);
        assert_eq!(&out[4..], &[[200, -200], [250, -250]]);
    }

    #[test]
    fn resampler_rate_is_correct_over_long_streams() {
        let src = 32768.0;
        let dst = 48000.0;
        let mut rs = Resampler::new(src, dst);
        let mut out = Vec::new();
        let mut fed = 0usize;
        for chunk in 0..200 {
            let n = 500 + (chunk % 7) * 13; // odd chunk sizes
            let input: Vec<[i16; 2]> = (0..n).map(|i| [(i % 200) as i16, 0]).collect();
            rs.process(&input, &mut out);
            fed += n;
        }
        let expected = fed as f64 * dst / src;
        assert!((out.len() as f64 - expected).abs() <= 2.0, "got {} expected {expected}", out.len());
    }

    #[test]
    fn resampler_downsample_count() {
        let mut rs = Resampler::new(65536.0, 32768.0);
        let mut out = Vec::new();
        let input: Vec<[i16; 2]> = (0..1000).map(|i| [i as i16, 0]).collect();
        rs.process(&input, &mut out);
        assert_eq!(out.len(), 500);
        // Starts on input[0], then every other input frame.
        assert_eq!(out[0], [0, 0]);
        assert_eq!(out[1], [2, 0]);
        assert_eq!(out[2], [4, 0]);
        assert_eq!(out[499], [998, 0]);
    }

    #[test]
    fn lerp_clamps_and_rounds() {
        assert_eq!(lerp(0, 10, 0.5), 5);
        assert_eq!(lerp(0, 1, 0.5), 1); // round half away from zero
        assert_eq!(lerp(i16::MIN, i16::MAX, 1.0), i16::MAX);
        assert_eq!(lerp(-100, 100, 0.0), -100);
    }

    #[test]
    fn fill_output_silence_before_prime_and_underrun_after() {
        let shared = Shared { ring: RingBuffer::new(8), underruns: AtomicU64::new(0), primed: AtomicBool::new(false), gain: AtomicU32::new(1.0f32.to_bits()) };
        let mut buf = vec![0.5f32; 8]; // 4 stereo frames
        fill_output(&mut buf, 2, &shared);
        assert!(buf.iter().all(|&s| s == 0.0));
        assert_eq!(shared.underruns.load(Ordering::Relaxed), 0, "not counted before priming");

        shared.primed.store(true, Ordering::Release);
        shared.ring.push([16384, -16384]);
        shared.ring.push([32767, 0]);
        fill_output(&mut buf, 2, &shared);
        assert!((buf[0] - 0.5).abs() < 1e-6);
        assert!((buf[1] + 0.5).abs() < 1e-6);
        assert!((buf[2] - 32767.0 / 32768.0).abs() < 1e-6);
        assert_eq!(buf[3], 0.0);
        assert!(buf[4..].iter().all(|&s| s == 0.0));
        assert_eq!(shared.underruns.load(Ordering::Relaxed), 1);

        // i16 devices with 6 channels: extra channels silent.
        let mut buf6 = vec![7i16; 12];
        shared.ring.push([-32768, 32767]);
        shared.ring.push([0, 0]);
        fill_output(&mut buf6, 6, &shared);
        assert_eq!(&buf6[..6], &[-32768, 32767, 0, 0, 0, 0]);
        assert_eq!(shared.underruns.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn gain_scales_the_callback_output() {
        let shared = Shared { ring: RingBuffer::new(8), underruns: AtomicU64::new(0), primed: AtomicBool::new(true), gain: AtomicU32::new(0.5f32.to_bits()) };
        shared.ring.push([16384, -32768]);
        let mut buf = vec![0.0f32; 2];
        fill_output(&mut buf, 2, &shared);
        assert!((buf[0] - 0.25).abs() < 1e-6);
        assert!((buf[1] + 0.5).abs() < 1e-6);
        shared.gain.store(0.0f32.to_bits(), Ordering::Relaxed);
        shared.ring.push([16384, 16384]);
        fill_output(&mut buf, 2, &shared);
        assert_eq!(buf, vec![0.0, 0.0]);
        assert_eq!(volume_gain(10), 1.0);
        assert_eq!(volume_gain(0), 0.0);
        assert!((volume_gain(5) - 0.25).abs() < 1e-6);
    }
}
