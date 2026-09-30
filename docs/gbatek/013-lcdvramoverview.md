# LCD VRAM Overview

Source: https://problemkaputt.de/gbatek.htm#lcdvramoverview

```
The GBA contains 96 Kbytes VRAM built-in, located at address 06000000-06017FFF,
depending on the BG Mode used as follows:

BG Mode 0,1,2 (Tile/Map based Modes)

  06000000-0600FFFF  64 KBytes shared for BG Map and Tiles
  06010000-06017FFF  32 KBytes OBJ Tiles

The shared 64K area can be split into BG Map area(s), and BG Tiles area(s), the
respective addresses for Map and Tile areas are set up by BG0CNT-BG3CNT
registers. The Map address may be specified in units of 2K (steps of 800h), the
Tile address in units of 16K (steps of 4000h).

BG Mode 0,1 (Tile/Map based Text mode)

The tiles may have 4bit or 8bit color depth, minimum map size is 32x32 tiles,
maximum is 64x64 tiles, up to 1024 tiles can be used per map.

  Item        Depth     Required Memory
  One Tile    4bit      20h bytes
  One Tile    8bit      40h bytes
  1024 Tiles  4bit      8000h (32K)
  1024 Tiles  8bit      10000h (64K) - excluding some bytes for BG map
  BG Map      32x32     800h (2K)
  BG Map      64x64     2000h (8K)

BG Mode 1,2 (Tile/Map based Rotation/Scaling mode)

The tiles may have 8bit color depth only, minimum map size is 16x16 tiles,
maximum is 128x128 tiles, up to 256 tiles can be used per map.

  Item        Depth     Required Memory
  One Tile    8bit      40h bytes
  256  Tiles  8bit      4000h (16K)
  BG Map      16x16     100h bytes
  BG Map      128x128   4000h (16K)

BG Mode 3 (Bitmap based Mode for still images)

  06000000-06013FFF  80 KBytes Frame 0 buffer (only 75K actually used)
  06014000-06017FFF  16 KBytes OBJ Tiles

BG Mode 4,5 (Bitmap based Modes)

  06000000-06009FFF  40 KBytes Frame 0 buffer (only 37.5K used in Mode 4)
  0600A000-06013FFF  40 KBytes Frame 1 buffer (only 37.5K used in Mode 4)
  06014000-06017FFF  16 KBytes OBJ Tiles

Note

Additionally to the above VRAM, the GBA also contains 1 KByte Palette RAM (at
05000000h) and 1 KByte OAM (at 07000000h) which are both used by the display
controller as well.
```
