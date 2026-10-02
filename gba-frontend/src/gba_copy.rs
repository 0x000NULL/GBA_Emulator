//! GBA-specific copy that the generic formatting rules (`ui::fmt`) leave to the app: save type
//! names.

use gba_core::SaveType;

/// Save type abbreviations for the cartridge label and the Bench head.
pub fn abbrev_save(t: SaveType) -> &'static str {
    match t {
        SaveType::None => "NONE",
        SaveType::Sram32K => "SRAM 32K",
        SaveType::Flash64K => "FL 64K",
        SaveType::Flash128K => "FL 128K",
        SaveType::Eeprom => "EEPROM",
    }
}

/// Longer save type names for the library details row.
pub fn save_name(t: SaveType) -> &'static str {
    match t {
        SaveType::None => "no save",
        SaveType::Sram32K => "SRAM 32K",
        SaveType::Flash64K => "FLASH 64K",
        SaveType::Flash128K => "FLASH 128K",
        SaveType::Eeprom => "EEPROM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_abbreviations() {
        assert_eq!(abbrev_save(SaveType::Flash128K), "FL 128K");
        assert_eq!(abbrev_save(SaveType::Eeprom), "EEPROM");
        assert_eq!(abbrev_save(SaveType::None), "NONE");
    }
}
