//! Cartridge header (0x000-0x0BF). Source: GBATEK "GBA Cartridge Header".

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    /// 0xA0: 12 bytes, space padded.
    pub title: String,
    /// 0xAC: 4 bytes, e.g. "B8CE".
    pub game_code: [u8; 4],
    /// 0xB0: 2 bytes, "01" = Nintendo.
    pub maker_code: [u8; 2],
    /// 0xBC
    pub version: u8,
    /// 0xBD, must match the computed complement for the BIOS to boot the cart.
    pub complement: u8,
    pub complement_ok: bool,
}

impl Header {
    pub fn parse(rom: &[u8]) -> Header {
        let title = String::from_utf8_lossy(&rom[0xA0..0xAC]).trim_end_matches(['\0', ' ']).to_string();
        let mut game_code = [0u8; 4];
        game_code.copy_from_slice(&rom[0xAC..0xB0]);
        let mut maker_code = [0u8; 2];
        maker_code.copy_from_slice(&rom[0xB0..0xB2]);
        let complement = rom[0xBD];
        let sum = rom[0xA0..0xBD].iter().fold(0u8, |a, &b| a.wrapping_sub(b)).wrapping_sub(0x19);
        Header { title, game_code, maker_code, version: rom[0xBC], complement, complement_ok: sum == complement }
    }

    pub fn game_code_str(&self) -> String {
        String::from_utf8_lossy(&self.game_code).into_owned()
    }
}
