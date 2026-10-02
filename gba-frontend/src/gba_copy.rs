//! GBA-specific copy that the generic code leaves to the app: save type names, and the save ids
//! the library stores (`LibraryEntry::save`, `index.toml`).

use gba_core::SaveType;

/// The library's save id for a save type.
pub fn save_kind(t: SaveType) -> &'static str {
    match t {
        SaveType::None => "none",
        SaveType::Sram32K => "sram32k",
        SaveType::Flash64K => "flash64k",
        SaveType::Flash128K => "flash128k",
        SaveType::Eeprom => "eeprom",
    }
}

/// The save type of a library save id (unknown ids: none).
pub fn save_type(s: &str) -> SaveType {
    match s {
        "sram32k" => SaveType::Sram32K,
        "flash64k" => SaveType::Flash64K,
        "flash128k" => SaveType::Flash128K,
        "eeprom" => SaveType::Eeprom,
        _ => SaveType::None,
    }
}

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
    fn save_type_strings_round_trip() {
        for t in [SaveType::None, SaveType::Sram32K, SaveType::Flash64K, SaveType::Flash128K, SaveType::Eeprom] {
            assert_eq!(save_type(save_kind(t)), t);
        }
    }

    #[test]
    fn save_abbreviations() {
        assert_eq!(abbrev_save(SaveType::Flash128K), "FL 128K");
        assert_eq!(abbrev_save(SaveType::Eeprom), "EEPROM");
        assert_eq!(abbrev_save(SaveType::None), "NONE");
    }
}
