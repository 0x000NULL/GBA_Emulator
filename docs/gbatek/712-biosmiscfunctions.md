# BIOS Misc Functions

Source: https://problemkaputt.de/gbatek.htm#biosmiscfunctions

```
GetBiosChecksum

WaitByLoop

GetCRC16

IsDebugger

GetSineTable

GetPitchTable

GetVolumeTable

CustomPost

GetBootProcs

SWI 0Dh (GBA) - GetBiosChecksum (Undocumented)

Calculates the checksum of the BIOS ROM (by reading in 32bit units, and adding
up these values). IRQ and FIQ are disabled during execution.

The checksum is BAAE187Fh (GBA and GBA SP), or BAAE1880h (NDS/3DS in GBA mode,
whereas the only difference is that the byte at [3F0Ch] is changed from 00h to
01h, otherwise the BIOS is 1:1 same as GBA BIOS, it does even include multiboot
code).

Parameters: None. Return: r0=Checksum.

SWI 03h (NDS7/NDS9/DSi7/DSi9) - WaitByLoop

Performs a "LOP: SUB R0,1 / BGT LOP" wait loop, the loop is executed in BIOS
memory, which provides reliable timings (regardless of the memory waitstates
& cache state of the calling procedure). Intended only for short delays
(eg. flash memory programming cycles).

  r0  Delay value (should be in range 1..7FFFFFFFh)

Execution time varies for ARM7 vs ARM9. On ARM9 it does also depend on whether
ROM is cached, and on DSi it does further depended on the ARM9 CPU clock, and
on whether using NDS or DSi BIOS ROM (NDS uses faster THUMB code, whilst DSi
uses ARM code, which is slow on uncached ARM9 ROM reads). For example, to get a
1 millisecond delay, use following values:

  CPU  Clock     Cache   BIOS     Value for 1ms
  ARM7 33.51MHz  none    NDS/DSi  r0=20BAh    ;=20BAh  ;-ARM7
  ARM9 67.03MHz  on      NDS/DSi  r0=20BAh*2  ;=4174h  ;\ARM9 with cache
  ARM9 134.06MHz on      DSi      r0=20BAh*4  ;=82E8h  ;/
  ARM9 67.03MHz  off     NDS      r0=20BAh/2  ;=105Dh  ;\
  ARM9 67.03MHz  off     DSi      r0=20BAh/4  ;=082Eh  ; ARM9 without cache
  ARM9 134.06MHz off     DSi      r0=20BAh/3  ;=0AE8h  ;/

Return: No return value.

SWI 0Eh (NDS7/NDS9/DSi7/DSi9) - GetCRC16

  r0  Initial CRC value (16bit, usually FFFFh)
  r1  Start Address   (must be aligned by 2)
  r2  Length in bytes (must be aligned by 2)

CRC16 checksums can be calculated as such:

  val[0..7] = C0C1h,C181h,C301h,C601h,CC01h,D801h,F001h,A001h
  for i=start to end
    crc=crc xor byte[i]
    for j=0 to 7
      crc=crc shr 1:if carry then crc=crc xor (val[j] shl (7-j))
    next j
  next i

Return:

  r0  Calculated 16bit CRC Value

Additionally, if the length is nonzero, r3 contains the last processed halfword
at [addr+len-2]. Unlike most other NDS7/DSi7 SWI functions (which do reject
reading from BIOS memory), this allows to dump the NDS7/DSi7 BIOS (except for
the memory region that is locked via BIOSPROT Port 4000308h).

SWI 0Fh (NDS7/NDS9) - IsDebugger

Detects if 4MB (normal) or 8MB (debug version) Main RAM installed.

Caution: Fails on ARM9 when cache is enabled (always returns 8MB state).

Return: r0 = result (0=normal console 4MB, 1=debug version 8MB)

Destroys halfword at [27FFFFAh] (NDS7) or [27FFFF8h] (NDS9)!

The SWI 0Fh function doesn't work stable if it gets interrupted by an interrupt
which is calling SWI 0Fh, which would destroy the above halfword scratch value
(unless the IRQ handler has saved/restored the halfword).

SWI 1Ah (NDS7/DSi7) - GetSineTable

  r0  Index (0..3Fh) (must be in that range, otherwise returns garbage)

Return: r0 = Desired Entry (0000h..7FF5h) ;SIN(0 .. 88.6 degrees)*8000h

SWI 1Bh (NDS7/DSi7) - GetPitchTable (DSi7: bugged)

  r0  Index (0..2FFh) (must be in that range, otherwise returns garbage)

BUG: DSi7 accidently reads from SineTable instead of PitchTable, as workaround
for obtaining PitchTable values, one can set "r0=(0..2FFh)-46Ah" on DSi.

Return: r0 = Desired Entry (0000h..FF8Ah) (unsigned)

SWI 1Ch (NDS7/DSi7) - GetVolumeTable

  r0  Index (0..2D3h) (must be in that range, otherwise returns garbage)

Return: r0 = Desired Entry (00h..7Fh) (unsigned)

SWI 1Fh (NDS9/DSi9) - CustomPost

Writes to the POSTFLG register, probably for use by Firmware boot procedure.

  r0  32bit value, to be written to POSTFLG, Port 4000300h

Return: No return value.

SWI 1Dh (NDS7/DSi7) - GetBootProcs

Returns addresses of Gamecart boot procedure/interrupt handler, probably for
use by Firmware boot procedure. Most of the returned NDS7 functions won't work
if the POSTFLG register is set.

The return values are somewhat XORed by each other (on DSi7 most of the values
are zero; which does rather negate the XORing effect, and, as a special
gimmick, one of the zero values is XORed by incoming r2).
```
