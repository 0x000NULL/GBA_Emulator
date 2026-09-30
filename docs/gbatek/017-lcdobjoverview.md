# LCD OBJ - Overview

Source: https://problemkaputt.de/gbatek.htm#lcdobjoverview

```
General

Objects (OBJs) are moveable sprites. Up to 128 OBJs (of any size, up to 64x64
dots each) can be displayed per screen, and under best circumstances up to 128
OBJs (of small 8x8 dots size) can be displayed per horizontal display line.

Maximum Number of Sprites per Line

The total available OBJ rendering cycles per line are

  1210  (=304*4-6)   If "H-Blank Interval Free" bit in DISPCNT register is 0
  954   (=240*4-6)   If "H-Blank Interval Free" bit in DISPCNT register is 1

The required rendering cycles are (depending on horizontal OBJ size)

  Cycles per <n> Pixels    OBJ Type              OBJ Type Screen Pixel Range
  n*1 cycles               Normal OBJs           8..64 pixels
  10+n*2 cycles            Rotation/Scaling OBJs 8..64 pixels   (area clipped)
  10+n*2 cycles            Rotation/Scaling OBJs 16..128 pixels (double size)

Caution:

The maximum number of OBJs per line is also affected by undisplayed (offscreen)
OBJs which are having higher priority than displayed OBJs.

To avoid this, move displayed OBJs to the begin of OAM memory (ie. OBJ0 has
highest priority, OBJ127 lowest).

Otherwise (in case that the program logic expects OBJs at fixed positions in
OAM) at least take care to set the OBJ size of undisplayed OBJs to 8x8 with
Rotation/Scaling disabled (this reduces the overload).

Does the above also apply for VERTICALLY OFFSCREEN (or VERTICALLY not on
CURRENT LINE) sprites ?

VRAM - Character Data

OBJs are always combined of one or more 8x8 pixel Tiles (much like BG Tiles in
BG Modes 0-2). However, OBJ Tiles are stored in a separate area in VRAM:
06010000-06017FFF (32 KBytes) in BG Mode 0-2, or 06014000-06017FFF (16 KBytes)
in BG Mode 3-5.

Depending on the size of the above area (16K or 32K), and on the OBJ color
depth (4bit or 8bit), 256-1024 8x8 dots OBJ Tiles can be defined.

OAM - Object Attribute Memory

This memory area contains Attributes which specify position, size, color depth,
etc. appearance for each of the 128 OBJs. Additionally, it contains 32 OBJ
Rotation/Scaling Parameter groups. OAM is located at 07000000-070003FF (sized 1
KByte).
```
