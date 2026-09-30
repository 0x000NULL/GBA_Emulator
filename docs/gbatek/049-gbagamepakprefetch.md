# GBA GamePak Prefetch

Source: https://problemkaputt.de/gbatek.htm#gbagamepakprefetch

```
GamePak Prefetch can be enabled in WAITCNT register. When prefetch buffer is
enabled, the GBA attempts to read opcodes from Game Pak ROM during periods when
the CPU is not using the bus (if any). Memory access is then performed with 0
Waits if the CPU requests data which is already stored in the buffer. The
prefetch buffer stores up to eight 16bit values.

GamePak ROM Opcodes

The prefetch feature works only with <opcodes> fetched from GamePak ROM.
Opcodes executed in RAM or BIOS are not affected by the prefetch feature (even
if that opcodes read <data> from GamePak ROM).

Prefetch Enable

For GamePak ROM opcodes, prefetch may occur in two situations:

  1) opcodes with internal cycles (I) which do not change R15, shift/rotate
     register-by-register, load opcodes (ldr,ldm,pop,swp), multiply opcodes
  2) opcodes that load/store memory (ldr,str,ldm,stm,etc.)

Prefetch Disable Bug

When Prefetch is disabled, the Prefetch Disable Bug will occur for all

  "Opcodes in GamePak ROM with Internal Cycles which do not change R15"

for those opcodes, the bug changes the opcode fetch time from 1S to 1N.

Note: Affected opcodes (with I cycles) are: Shift/rotate register-by-register
opcodes, multiply opcodes, and load opcodes (ldr,ldm,pop,swp).
```
