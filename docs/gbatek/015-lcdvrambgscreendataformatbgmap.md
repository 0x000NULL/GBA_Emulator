# LCD VRAM BG Screen Data Format (BG Map)

Source: https://problemkaputt.de/gbatek.htm#lcdvrambgscreendataformatbgmap

```
The display background consists of 8x8 dot tiles, the arrangement of these
tiles is specified by the BG Screen Data (BG Map). The separate entries in this
map are as follows:

Text BG Screen (2 bytes per entry)

Specifies the tile number and attributes. Note that BG tile numbers are always
specified in steps of 1 (unlike OBJ tile numbers which are using steps of two
in 256 color/1 palette mode).

  Bit   Expl.
  0-9   Tile Number     (0-1023) (a bit less in 256 color mode, because
                           there'd be otherwise no room for the bg map)
  10    Horizontal Flip (0=Normal, 1=Mirrored)
  11    Vertical Flip   (0=Normal, 1=Mirrored)
  12-15 Palette Number  (0-15)    (Not used in 256 color/1 palette mode)

A Text BG Map always consists of 32x32 entries (256x256 pixels), 400h entries =
800h bytes. However, depending on the BG Size, one, two, or four of these Maps
may be used together, allowing to create backgrounds of 256x256, 512x256,
256x512, or 512x512 pixels, if so, the first map (SC0) is located at base+0,
the next map (SC1) at base+800h, and so on.

Rotation/Scaling BG Screen (1 byte per entry)

In this mode, only 256 tiles can be used. There are no x/y-flip attributes, the
color depth is always 256 colors/1 palette.

  Bit   Expl.
  0-7   Tile Number     (0-255)

The dimensions of Rotation/Scaling BG Maps depend on the BG size. For size 0-3
that are: 16x16 tiles (128x128 pixels), 32x32 tiles (256x256 pixels), 64x64
tiles (512x512 pixels), or 128x128 tiles (1024x1024 pixels).

The size and VRAM base address of the separate BG maps for BG0-3 are set up by
BG0CNT-BG3CNT registers.
```
