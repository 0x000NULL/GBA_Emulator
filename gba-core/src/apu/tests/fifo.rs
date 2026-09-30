//! DirectSound FIFOs: ordering, refill threshold, underruns, timer routing, reset bits.

use super::*;
use crate::apu::fifo::Fifo;
use crate::apu::FifoRequest;

#[test]
fn pop_order_and_refill_threshold() {
    let mut f = Fifo::default();
    for i in 0..8u8 {
        f.push(&[i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 3]);
    }
    assert_eq!(f.len(), 32);
    for i in 0..15u8 {
        assert!(!f.pop(), "17+ bytes left: no refill request (pop {i})");
        assert_eq!(f.sample as u8, i);
    }
    assert_eq!(f.len(), 17);
    assert!(f.pop(), "16 bytes left: request refill");
    assert_eq!(f.sample, 15);
    assert_eq!(f.len(), 16);
    for i in 16..32u8 {
        assert!(f.pop());
        assert_eq!(f.sample as u8, i);
    }
    assert!(f.is_empty());
    assert_eq!(f.underruns, 0);
}

#[test]
fn underrun_counts_and_repeats_last_sample() {
    let mut f = Fifo::default();
    f.push(&[0x85]);
    assert!(f.pop());
    assert_eq!(f.sample, 0x85u8 as i8);
    assert_eq!(f.underruns, 0);
    assert!(f.pop());
    assert_eq!(f.underruns, 1);
    assert_eq!(f.sample, 0x85u8 as i8, "empty FIFO repeats the last sample");
    f.pop();
    assert_eq!(f.underruns, 2);
}

#[test]
fn push_drops_bytes_beyond_32() {
    let mut f = Fifo::default();
    let data: Vec<u8> = (0..40).collect();
    f.push(&data);
    assert_eq!(f.len(), 32);
    for i in 0..32u8 {
        f.pop();
        assert_eq!(f.sample as u8, i);
    }
}

#[test]
fn reset_empties_and_zeroes_sample() {
    let mut f = Fifo::default();
    f.push(&[1, 2, 3]);
    f.pop();
    f.reset();
    assert!(f.is_empty());
    assert_eq!(f.sample, 0);
    f.push(&[9]);
    f.pop();
    assert_eq!(f.sample, 9);
}

#[test]
fn byte_halfword_and_word_writes_preserve_order() {
    let (mut a, mut s) = apu_on();
    a.fifo_write(0, &0x0403_0201u32.to_le_bytes());
    a.fifo_write(0, &0x0605u16.to_le_bytes());
    w8(&mut a, &mut s, 0xA0, 7);
    w8(&mut a, &mut s, 0xA1, 8);
    a.fifo_write(1, &[0x11, 0x22]);
    assert_eq!(a.fifo_a.len(), 8);
    assert_eq!(a.fifo_b.len(), 2);
    for i in 1..=8i8 {
        a.fifo_a.pop();
        assert_eq!(a.fifo_a.sample, i);
    }
    a.fifo_b.pop();
    assert_eq!(a.fifo_b.sample, 0x11);
}

#[test]
fn timer_select_routes_pops_and_requests() {
    let (mut a, mut s) = apu_on();
    // A on timer 0, B on timer 1, both enabled L+R, 100% volumes.
    w16(&mut a, &mut s, 0x82, 0x730F);
    assert_eq!(r16(&a, 0x82), 0x730F);
    a.fifo_write(0, &[1, 2, 3, 4]);
    a.fifo_write(1, &[5, 6, 7, 8]);
    assert_eq!(a.on_timer_overflow(0), FifoRequest { a: true, b: false });
    assert_eq!(a.fifo_a.sample, 1);
    assert_eq!(a.fifo_b.sample, 0);
    assert_eq!(a.on_timer_overflow(1), FifoRequest { a: false, b: true });
    assert_eq!(a.fifo_b.sample, 5);
    assert_eq!(a.on_timer_overflow(2), FifoRequest::default());
    assert_eq!(a.on_timer_overflow(3), FifoRequest::default());
    assert_eq!(a.fifo_a.len() + a.fifo_b.len(), 6);
    // Both on timer 1 (high byte 0x77: A R/L/timer1, B R/L/timer1, no reset bits).
    w8(&mut a, &mut s, 0x83, 0x77);
    assert_eq!(a.on_timer_overflow(0), FifoRequest::default());
    assert_eq!(a.on_timer_overflow(1), FifoRequest { a: true, b: true });
    assert_eq!((a.fifo_a.sample, a.fifo_b.sample), (2, 6));
}

#[test]
fn no_request_while_more_than_16_bytes_remain() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x82, 0x4304); // A on timer 0, B on timer 1
    let data: Vec<u8> = (0..32).collect();
    a.fifo_write(0, &data);
    for _ in 0..15 {
        assert_eq!(a.on_timer_overflow(0), FifoRequest::default());
    }
    assert_eq!(a.on_timer_overflow(0), FifoRequest { a: true, b: false });
    assert_eq!(a.fifo_a.len(), 16);
}

#[test]
fn master_off_blocks_fifo_pops() {
    let mut a = Apu::new();
    let mut s = Scheduler::new();
    w16(&mut a, &mut s, 0x82, 0x4304); // A on timer 0, B on timer 1
    a.fifo_write(0, &[1, 2, 3, 4]);
    assert_eq!(a.on_timer_overflow(0), FifoRequest::default());
    assert_eq!(a.fifo_a.len(), 4);
    assert_eq!(a.fifo_a.underruns, 0);
    w8(&mut a, &mut s, 0x84, 0x80);
    assert_eq!(a.on_timer_overflow(0), FifoRequest { a: true, b: false });
    assert_eq!(a.fifo_a.sample, 1);
}

#[test]
fn soundcnt_h_reset_bits_clear_fifos_and_read_zero() {
    let (mut a, mut s) = apu_on();
    a.fifo_write(0, &[1, 2, 3, 4]);
    a.fifo_write(1, &[5, 6, 7, 8]);
    a.fifo_a.pop();
    a.fifo_b.pop();
    w16(&mut a, &mut s, 0x82, 0x0800 | 0x0304);
    assert!(a.fifo_a.is_empty());
    assert_eq!(a.fifo_a.sample, 0);
    assert_eq!(a.fifo_b.len(), 3);
    assert_eq!(a.fifo_b.sample, 5);
    assert_eq!(r16(&a, 0x82), 0x0304, "reset bit reads back 0");
    w8(&mut a, &mut s, 0x83, 0x83);
    assert!(a.fifo_b.is_empty());
    assert_eq!(a.fifo_b.sample, 0);
    assert_eq!(r16(&a, 0x82), 0x0304);
}

#[test]
fn underruns_surface_through_timer_path() {
    let (mut a, mut s) = apu_on();
    w16(&mut a, &mut s, 0x82, 0x4304); // A on timer 0, B on timer 1
    a.fifo_write(0, &[0x40]);
    a.on_timer_overflow(0);
    a.on_timer_overflow(0);
    a.on_timer_overflow(0);
    assert_eq!(a.fifo_a.underruns, 2);
    assert_eq!(a.fifo_a.sample, 0x40);
    assert_eq!(a.fifo_b.underruns, 0, "B is on timer 1 here");
}

#[test]
fn unrouted_fifo_pops_do_not_count_as_underruns() {
    let (mut a, mut s) = apu_on();
    // A routed L+R on timer 0; B not routed anywhere but also clocked by timer 0 (bit 14 = 0).
    w16(&mut a, &mut s, 0x82, 0x0304);
    for _ in 0..10 {
        a.on_timer_overflow(0);
    }
    assert_eq!(a.fifo_a.underruns, 10, "routed FIFO counts underruns");
    assert_eq!(a.fifo_b.underruns, 0, "unrouted FIFO pops silently");
    // Route B to the left speaker: its underruns count from now on.
    w16(&mut a, &mut s, 0x82, 0x2304);
    a.on_timer_overflow(0);
    assert_eq!(a.fifo_b.underruns, 1);
    // An unrouted FIFO still pops data (the timer clocks it regardless).
    w16(&mut a, &mut s, 0x82, 0x0304);
    a.fifo_write(1, &[7, 8]);
    a.on_timer_overflow(0);
    assert_eq!(a.fifo_b.sample, 7);
    assert_eq!(a.fifo_b.len(), 1);
}
