# GBA Cart Protections

Source: https://problemkaputt.de/gbatek.htm#gbacartprotections

```
Classic NES Series

These are some NES/Famicom games ported or emulated to work on GBA. The games
are doing some uncommon stuff that can cause compatibility problems when not
using original GBA consoles or cartridges.

- CPU pipeline (selfmodifying code that shall NOT affect prefetched opcodes)

- STMDA write to I/O ports (writes in INCREASING order, not DECREASING order)

- SRAM detection (refuses to run if SRAM exists; the games do contain EEPROM)

- ROM mirrors (instead of the usual increasing numbers in unused ROM area)

- RAM mirrors (eg. main RAM accessed at 2F00000h instead of 2000000h)

Note: These games can be detected by checking [80000ACh]="F" (ie. game
code="Fxxx").
```
