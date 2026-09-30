# BIOS Memory Copy

Source: https://problemkaputt.de/gbatek.htm#biosmemorycopy

```
CpuFastSet

CpuSet

SWI 0Ch (GBA/NDS7/NDS9/DSi7/DSi9) - CpuFastSet

Memory copy/fill in units of 32 bytes. Memcopy is implemented as repeated
LDMIA/STMIA [Rb]!,r2-r9 instructions. Memfill as single LDR followed by
repeated STMIA [Rb]!,r2-r9.

After processing all 32-byte-blocks, the NDS/DSi additonally processes the
remaining words as 4-byte blocks. BUG: The NDS/DSi uses the fast 32-byte-block
processing only for the first N bytes (not for the first N words), so only the
first quarter of the memory block is FAST, the remaining three quarters are
SLOWLY copied word-by-word.

The length is specifed as wordcount, ie. the number of bytes divided by 4.

On the GBA, the length should be a multiple of 8 words (32 bytes) (otherwise
the GBA is forcefully rounding-up the length). On NDS/DSi, the length may be
any number of words (4 bytes).

  r0    Source address        (must be aligned by 4)
  r1    Destination address   (must be aligned by 4)
  r2    Length/Mode
          Bit 0-20  Wordcount (GBA: rounded-up to multiple of 8 words)
          Bit 24    Fixed Source Address (0=Copy, 1=Fill by WORD[r0])

Return: No return value, Data written to destination address.

SWI 0Bh (GBA/NDS7/NDS9/DSi7/DSi9) - CpuSet

Memory copy/fill in units of 4 bytes or 2 bytes. Memcopy is implemented as
repeated LDMIA/STMIA [Rb]!,r3 or LDRH/STRH r3,[r0,r5] instructions. Memfill as
single LDMIA or LDRH followed by repeated STMIA [Rb]!,r3 or STRH r3,[r0,r5].

The length must be a multiple of 4 bytes (32bit mode) or 2 bytes (16bit mode).
The (half)wordcount in r2 must be length/4 (32bit mode) or length/2 (16bit
mode), ie. length in word/halfword units rather than byte units.

  r0    Source address        (must be aligned by 4 for 32bit, by 2 for 16bit)
  r1    Destination address   (must be aligned by 4 for 32bit, by 2 for 16bit)
  r2    Length/Mode
          Bit 0-20  Wordcount (for 32bit), or Halfwordcount (for 16bit)
          Bit 24    Fixed Source Address (0=Copy, 1=Fill by {HALF}WORD[r0])
          Bit 26    Datasize (0=16bit, 1=32bit)

Return: No return value, Data written to destination address.

Note: On GBA, NDS7 and DSi7, these two functions will silently reject to do
anything if the source start or end addresses are reaching into the BIOS area.
The NDS9 and DSi9 don't have such read-proctections.
```
