# LCD OBJ - OAM Attributes

Source: https://problemkaputt.de/gbatek.htm#lcdobjoamattributes

```
OBJ Attributes

There are 128 entries in OAM for each OBJ0-OBJ127. Each entry consists of 6
bytes (three 16bit Attributes). Attributes for OBJ0 are located at 07000000,
for OBJ1 at 07000008, OBJ2 at 07000010, and so on.

As you can see, there are blank spaces at 07000006, 0700000E, 07000016, etc. -
these 16bit values are used for OBJ Rotation/Scaling (as described in the next
chapter) - they are not directly related to the separate OBJs.

OBJ Attribute 0 (R/W)

  Bit   Expl.
  0-7   Y-Coordinate           (0-255)
  8     Rotation/Scaling Flag  (0=Off, 1=On)
  When Rotation/Scaling used (Attribute 0, bit 8 set):
    9     Double-Size Flag     (0=Normal, 1=Double)
  When Rotation/Scaling not used (Attribute 0, bit 8 cleared):
    9     OBJ Disable          (0=Normal, 1=Not displayed)
  10-11 OBJ Mode  (0=Normal, 1=Semi-Transparent, 2=OBJ Window, 3=Prohibited)
  12    OBJ Mosaic             (0=Off, 1=On)
  13    Colors/Palettes        (0=16/16, 1=256/1)
  14-15 OBJ Shape              (0=Square,1=Horizontal,2=Vertical,3=Prohibited)

Caution: A very large OBJ (of 128 pixels vertically, ie. a 64 pixels OBJ in a
Double Size area) located at Y>128 will be treated as at Y>-128, the OBJ
is then displayed parts offscreen at the TOP of the display, it is then NOT
displayed at the bottom.

OBJ Attribute 1 (R/W)

  Bit   Expl.
  0-8   X-Coordinate           (0-511)
  When Rotation/Scaling used (Attribute 0, bit 8 set):
    9-13  Rotation/Scaling Parameter Selection (0-31)
          (Selects one of the 32 Rotation/Scaling Parameters that
          can be defined in OAM, for details read next chapter.)
  When Rotation/Scaling not used (Attribute 0, bit 8 cleared):
    9-11  Not used
    12    Horizontal Flip      (0=Normal, 1=Mirrored)
    13    Vertical Flip        (0=Normal, 1=Mirrored)
  14-15 OBJ Size               (0..3, depends on OBJ Shape, see Attr 0)
          Size  Square   Horizontal  Vertical
          0     8x8      16x8        8x16
          1     16x16    32x8        8x32
          2     32x32    32x16       16x32
          3     64x64    64x32       32x64

OBJ Attribute 2 (R/W)

  Bit   Expl.
  0-9   Character Name          (0-1023=Tile Number)
  10-11 Priority relative to BG (0-3; 0=Highest)
  12-15 Palette Number   (0-15) (Not used in 256 color/1 palette mode)

Notes:

OBJ Mode

The OBJ Mode may be Normal, Semi-Transparent, or OBJ Window.

Semi-Transparent means that the OBJ is used as 'Alpha Blending 1st Target'
(regardless of BLDCNT register, for details see chapter about Color Special
Effects).

OBJ Window means that the OBJ is not displayed, instead, dots with non-zero
color are used as mask for the OBJ Window, see DISPCNT and WINOUT for details.

OBJ Tile Number

There are two situations which may divide the amount of available tiles by two
(by four if both situations apply):

1. When using the 256 Colors/1 Palette mode, only each second tile may be used,
the lower bit of the tile number should be zero (in 2-dimensional mapping mode,
the bit is completely ignored).

2. When using BG Mode 3-5 (Bitmap Modes), only tile numbers 512-1023 may be
used. That is because lower 16K of OBJ memory are used for BG. Attempts to use
tiles 0-511 are ignored (not displayed).

Priority

In case that the 'Priority relative to BG' is the same than the priority of one
of the background layers, then the OBJ becomes higher priority and is displayed
on top of that BG layer.

Caution: Take care not to mess up BG Priority and OBJ priority. For example,
the following would cause garbage to be displayed:

  OBJ No. 0 with Priority relative to BG=1   ;hi OBJ prio, lo BG prio
  OBJ No. 1 with Priority relative to BG=0   ;lo OBJ prio, hi BG prio

That is, OBJ0 is always having priority above OBJ1-127, so assigning a lower BG
Priority to OBJ0 than for OBJ1-127 would be a bad idea.
```
