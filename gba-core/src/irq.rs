//! IE / IF / IME / HALTCNT. Source: GBATEK "GBA Interrupt Control".

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Irq {
    VBlank = 1 << 0,
    HBlank = 1 << 1,
    VCount = 1 << 2,
    Timer0 = 1 << 3,
    Timer1 = 1 << 4,
    Timer2 = 1 << 5,
    Timer3 = 1 << 6,
    Serial = 1 << 7,
    Dma0 = 1 << 8,
    Dma1 = 1 << 9,
    Dma2 = 1 << 10,
    Dma3 = 1 << 11,
    Keypad = 1 << 12,
    GamePak = 1 << 13,
}

impl Irq {
    pub fn timer(n: u8) -> Irq {
        match n & 3 {
            0 => Irq::Timer0,
            1 => Irq::Timer1,
            2 => Irq::Timer2,
            _ => Irq::Timer3,
        }
    }
    pub fn dma(n: u8) -> Irq {
        match n & 3 {
            0 => Irq::Dma0,
            1 => Irq::Dma1,
            2 => Irq::Dma2,
            _ => Irq::Dma3,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct IrqController {
    /// 4000200h IE
    pub ie: u16,
    /// 4000202h IF (write 1 to acknowledge)
    pub if_: u16,
    /// 4000208h IME bit 0
    pub ime: bool,
    /// HALTCNT: CPU halted until (IE & IF) != 0 (IME not required).
    pub halted: bool,
    /// HALTCNT bit 7: stop mode (until keypad/serial/gamepak IRQ).
    pub stopped: bool,
}

impl IrqController {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn raise(&mut self, irq: Irq) {
        self.if_ |= irq as u16;
    }

    /// True when the CPU should take an IRQ exception (CPSR.I is the CPU's business).
    #[inline]
    pub fn pending(&self) -> bool {
        self.ime && (self.ie & self.if_) != 0
    }

    /// True when a halted CPU must wake (IME is ignored for wake-up).
    #[inline]
    pub fn wake_condition(&self) -> bool {
        (self.ie & self.if_) != 0
    }

    pub fn write_if_ack(&mut self, v: u16) {
        self.if_ &= !v;
    }

    /// 4000301h HALTCNT: bit 7 = 0 halt, 1 stop.
    pub fn write_haltcnt(&mut self, v: u8) {
        if v & 0x80 != 0 {
            self.stopped = true;
        } else {
            self.halted = true;
        }
    }
}
