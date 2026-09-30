# LCD VRAM Bitmap BG Modes

Source: https://problemkaputt.de/gbatek.htm#lcdvrambitmapbgmodes

```
In BG Modes 3-5 the background is defined in form of a bitmap (unlike as for
Tile/Map based BG modes). Bitmaps are implemented as BG2, with Rotation/Scaling
support. As bitmap modes are occupying 80KBytes of BG memory, only 16KBytes of
VRAM can be used for OBJ tiles.

BG Mode 3 - 240x160 pixels, 32768 colors

Two bytes are associated to each pixel, directly defining one of the 32768
colors (without using palette data, and thus not supporting a 'transparent' BG
color).

  Bit   Expl.
  0-4   Red Intensity   (0-31)
  5-9   Green Intensity (0-31)
  10-14 Blue Intensity  (0-31)
  15    Not used in GBA Mode (in NDS Mode: Alpha=0=Transparent, Alpha=1=Normal)

The first 480 bytes define the topmost line, the next 480 the next line, and so
on. The background occupies 75 KBytes (06000000-06012BFF), most of the 80
Kbytes BG area, not allowing to redraw an invisible second frame in background,
so this mode is mostly recommended for still images only.

BG Mode 4 - 240x160 pixels, 256 colors (out of 32768 colors)

One byte is associated to each pixel, selecting one of the 256 palette entries.
Color 0 (backdrop) is transparent, and OBJs may be displayed behind the bitmap.

The first 240 bytes define the topmost line, the next 240 the next line, and so
on. The background occupies 37.5 KBytes, allowing two frames to be used
(06000000-060095FF for Frame 0, and 0600A000-060135FF for Frame 1).

BG Mode 5 - 160x128 pixels, 32768 colors

Colors are defined as for Mode 3 (see above), but horizontal and vertical size
are cut down to 160x128 pixels only - smaller than the physical dimensions of
the LCD screen.

The background occupies exactly 40 KBytes, so that BG VRAM may be split into
two frames (06000000-06009FFF for Frame 0, and 0600A000-06013FFF for Frame 1).

In BG modes 4,5, one Frame may be displayed (selected by DISPCNT Bit 4), the
other Frame is invisible and may be redrawn in background.
```
