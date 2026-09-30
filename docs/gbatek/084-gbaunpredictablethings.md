# GBA Unpredictable Things

Source: https://problemkaputt.de/gbatek.htm#gbaunpredictablethings

```
Forward

Most of the below is caused by 'traces' from previous operations which have
used the databus. No promises that the results are stable on all current or
future GBA models, and/or under all temperature and interference circumstances.

Also, below specifies 32bit data accesses only. When reading units less than
32bit, data is rotated depending on the alignment of the originally specified
address, and 8bit or 16bit are then isolated from the 32bit value as usually.

Reading from BIOS Memory (00000000-00003FFF)

The BIOS memory is protected against reading, the GBA allows to read opcodes or
data only if the program counter is located inside of the BIOS area. If the
program counter is not in the BIOS area, reading will return the most recent
successfully fetched BIOS opcode (eg. the opcode at [00DCh+8] after startup and
SoftReset, the opcode at [0134h+8] during IRQ execution, and opcode at
[013Ch+8] after IRQ execution, and opcode at [0188h+8] after SWI execution).

Reading from Unused Memory (00004000-01FFFFFF,10000000-FFFFFFFF)

Accessing unused memory at 00004000h-01FFFFFFh, and 10000000h-FFFFFFFFh (and
02000000h-03FFFFFFh when RAM is disabled via Port 4000800h) returns the
recently pre-fetched opcode. For ARM code this is simply:

  WORD = [$+8]

For THUMB code the result consists of two 16bit fragments and depends on the
address area and alignment where the opcode was stored.

For THUMB code in Main RAM, Palette Memory, VRAM, and Cartridge ROM this is:

  LSW = [$+4], MSW = [$+4]

For THUMB code in BIOS or OAM (and in 32K-WRAM on Original-NDS (in GBA mode)):

  LSW = [$+4], MSW = [$+6]   ;for opcodes at 4-byte aligned locations
  LSW = [$+2], MSW = [$+4]   ;for opcodes at non-4-byte aligned locations

For THUMB code in 32K-WRAM on GBA, GBA SP, GBA Micro, NDS-Lite (but not NDS):

  LSW = [$+4], MSW = OldHI   ;for opcodes at 4-byte aligned locations
  LSW = OldLO, MSW = [$+4]   ;for opcodes at non-4-byte aligned locations

Whereas OldLO/OldHI are usually:

  OldLO=[$+2], OldHI=[$+2]

Unless the previous opcode's prefetch was overwritten; that can happen if the
previous opcode was itself an LDR opcode, ie. if it was itself reading data:

  OldLO=LSW(data), OldHI=MSW(data)
  Theoretically, this might also change if a DMA transfer occurs.

Note: Additionally, as usually, the 32bit data value will be rotated if the
data address wasn't 4-byte aligned, and the upper bits of the 32bit value will
be masked in case of LDRB/LDRH reads.

Note: The opcode prefetch is caused by the prefetch pipeline in the CPU itself,
not by the external gamepak prefetch, ie. it works for code in ROM and RAM as
well.

Reading from Unused or Write-Only I/O Ports

Works like above Unused Memory when the entire 32bit memory fragment is Unused
(eg. 0E0h) and/or Write-Only (eg. DMA0SAD). And otherwise, returns zero if the
lower 16bit fragment is readable (eg. 04Ch=MOSAIC, 04Eh=NOTUSED/ZERO).

Reading from GamePak ROM when no Cartridge is inserted

Because Gamepak uses the same signal-lines for both 16bit data and for lower
16bit halfword address, the entire gamepak ROM area is effectively filled by
incrementing 16bit values (Address/2 AND FFFFh).

Memory Mirrors

Most internal memory is mirrored across the whole 24bit/16MB address space in
which it is located: Slow On-board RAM at 2XXXXXX, Fast On-Chip RAM at
3XXXXXXh, Palette RAM at 5XXXXXXh, VRAM at 6XXXXXXh, and OAM at 7XXXXXXh. Even
though VRAM is sized 96K (64K+32K), it is repeated in steps of 128K
(64K+32K+32K, the two 32K blocks itself being mirrors of each other).

BIOS ROM, Normal ROM Cartridges, and I/O area are NOT mirrored, the only
exception is the undocumented I/O port at 4000800h (repeated each 64K).

The 64K SRAM area is mirrored across the whole 32MB area at E000000h-FFFFFFFh,
also, inside of the 64K SRAM field, 32K SRAM chips are repeated twice.

Writing 8bit Data to Video Memory

Video Memory (BG, OBJ, OAM, Palette) can be written to in 16bit and 32bit units
only. Attempts to write 8bit data (by STRB opcode) won't work:

Writes to OBJ (6010000h-6017FFFh) (or 6014000h-6017FFFh in Bitmap mode) and to
OAM (7000000h-70003FFh) are ignored, the memory content remains unchanged.

Writes to BG (6000000h-600FFFFh) (or 6000000h-6013FFFh in Bitmap mode) and to
Palette (5000000h-50003FFh) are writing the new 8bit value to BOTH upper and
lower 8bits of the addressed halfword, ie. "[addr AND NOT 1]=data*101h".

Using Invalid Tile Numbers

In Text mode, large tile numbers (combined with a non-zero character base
setting in BGnCNT register) may exceed the available 64K of BG VRAM.

On GBA and GBA SP, such invalid tiles are displayed as if the character data is
filled by the 16bit BG Map entry value (ie. as vertically striped tiles). Above
applies only if there is only one BG layer enabled, with two or more layers,
things are getting much more complicated: tile-data is then somehow derived
from the other layers, depending on their priority order and scrolling offsets.

On NDS (in GBA mode), such invalid tiles are displayed as if the character data
is zero-filled (ie. as invisible/transparent tiles).

Accessing SRAM Area by 16bit/32bit

Reading retrieves 8bit value from specified address, multiplied by 0101h (LDRH)
or by 01010101h (LDR). Writing changes the 8bit value at the specified address
only, being set to LSB of (source_data ROR (address*8)).
```
