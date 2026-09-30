# GBA Cartridge ROM

Source: https://problemkaputt.de/gbatek.htm#gbacartridgerom

```
ROM Size

The games F-ZERO and Super Mario Advance use ROMs of 4 MBytes each. Zelda uses
8 MBytes. Not sure if other sizes are manufactured.

ROM Waitstates

The GBA starts the cartridge with 4,2 waitstates (N,S) and prefetch disabled.
The program may change these settings by writing to WAITCNT, the games F-ZERO
and Super Mario Advance use 3,1 waitstates (N,S) each, with prefetch enabled.

Third-party flashcards are reportedly running unstable with these settings.
Also, prefetch and shorter waitstates are allowing to read more data and
opcodes from ROM is less time, the downside is that it increases the power
consumption.

ROM Chip

Because of how 24bit addresses are squeezed through the Gampak bus, the
cartridge must include a circuit that latches the lower 16 address bits on
non-sequential access, and that increments these bits on sequential access.
Nintendo includes this circuit directly in the ROM chip.

Also, the ROM must have 16bit data bus (or a circuit which converts two 8bit
data units into one 16bit unit - by not exceeding the waitstate timings).
```
