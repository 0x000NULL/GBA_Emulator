//! FLASH 64K / 128K backup. Source: GBATEK "GBA Cart Backup Flash ROM".
//! Command sequence: [5555]=AA, [2AAA]=55, [5555]=cmd. 128K parts bank-switch via [0000]=bank
//! after command B0. IDs: 64K Panasonic 0x1B32 (we use Panasonic/SST 0xD4BF), 128K Sanyo 0x1362.
//! M0: full command state machine; verified by jsmolka flash64/flash128 in M5.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ready,
    Cmd1,
    Cmd2,
    Erase1,
    Erase2,
    EraseCmd,
    /// Next write is a byte program.
    Program,
    /// Next write to 0000 selects the bank.
    BankSelect,
}

pub struct Flash {
    data: Vec<u8>,
    is_128k: bool,
    bank: usize,
    state: State,
    id_mode: bool,
}

impl Flash {
    pub fn new(is_128k: bool) -> Self {
        let size = if is_128k { 0x2_0000 } else { 0x1_0000 };
        Flash { data: vec![0xFF; size], is_128k, bank: 0, state: State::Ready, id_mode: false }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn load(&mut self, d: &[u8]) {
        let n = d.len().min(self.data.len());
        self.data[..n].copy_from_slice(&d[..n]);
    }

    fn id(&self) -> [u8; 2] {
        if self.is_128k {
            [0x62, 0x13] // Sanyo
        } else {
            [0xBF, 0xD4] // SST
        }
    }

    pub fn peek8(&self, addr: u16) -> u8 {
        if self.id_mode && addr < 2 {
            return self.id()[addr as usize];
        }
        self.data[self.bank * 0x1_0000 + addr as usize]
    }

    pub fn read8(&mut self, addr: u16) -> u8 {
        self.peek8(addr)
    }

    /// Returns true when the backing data changed.
    pub fn write8(&mut self, addr: u16, v: u8) -> bool {
        match self.state {
            State::Ready | State::EraseCmd if addr == 0x5555 && v == 0xAA => {
                self.state = if self.state == State::EraseCmd { State::Erase1 } else { State::Cmd1 };
                false
            }
            State::Cmd1 if addr == 0x2AAA && v == 0x55 => {
                self.state = State::Cmd2;
                false
            }
            State::Erase1 if addr == 0x2AAA && v == 0x55 => {
                self.state = State::Erase2;
                false
            }
            State::Cmd2 if addr == 0x5555 => {
                self.state = State::Ready;
                match v {
                    0x90 => self.id_mode = true,
                    0xF0 => self.id_mode = false,
                    0x80 => self.state = State::EraseCmd,
                    0xA0 => self.state = State::Program,
                    0xB0 if self.is_128k => self.state = State::BankSelect,
                    _ => {}
                }
                false
            }
            State::Erase2 => {
                self.state = State::Ready;
                if addr == 0x5555 && v == 0x10 {
                    self.data.fill(0xFF);
                    return true;
                }
                if v == 0x30 {
                    let base = self.bank * 0x1_0000 + (addr as usize & 0xF000);
                    self.data[base..base + 0x1000].fill(0xFF);
                    return true;
                }
                false
            }
            State::Program => {
                self.state = State::Ready;
                let i = self.bank * 0x1_0000 + addr as usize;
                self.data[i] &= v;
                true
            }
            State::BankSelect if addr == 0 => {
                self.state = State::Ready;
                self.bank = (v & 1) as usize;
                false
            }
            _ => {
                self.state = State::Ready;
                false
            }
        }
    }
}
