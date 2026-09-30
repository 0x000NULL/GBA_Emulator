# GBA Cart Backup IDs

Source: https://problemkaputt.de/gbatek.htm#gbacartbackupids

```
Nintendo didn't include a backup-type entry in the ROM header, however, the
required type can be detected by ID strings in the ROM-image. Nintendo's tools
are automatically inserting these strings (as part of their library headers).
When using other tools, you may insert ID strings by hand.

ID Strings

The ID string must be located at a word-aligned memory location, the string
length should be a multiple of 4 bytes (padded with zero's).

  EEPROM_Vnnn    EEPROM 512 bytes or 8 Kbytes (4Kbit or 64Kbit)
  SRAM_Vnnn      SRAM 32 Kbytes (256Kbit)
  FLASH_Vnnn     FLASH 64 Kbytes (512Kbit) (ID used in older files)
  FLASH512_Vnnn  FLASH 64 Kbytes (512Kbit) (ID used in newer files)
  FLASH1M_Vnnn   FLASH 128 Kbytes (1Mbit)

For Nintendo's tools, "nnn" is a 3-digit library version number. When using
other tools, best keep it set to "nnn" rather than inserting numeric digits.

Notes

No$gba does auto-detect most backup types, even without ID strings, except for
128K FLASH (without ID "FLASH1M_Vnnn", the FLASH size defaults to 64K).
Ideally, for faster detection, the ID should be put into the first some bytes
of the ROM-image (ie. somewhere right after the ROM header).
```
