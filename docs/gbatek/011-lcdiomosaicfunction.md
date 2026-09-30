# LCD I/O Mosaic Function

Source: https://problemkaputt.de/gbatek.htm#lcdiomosaicfunction

```
400004Ch - MOSAIC - Mosaic Size (W)

The Mosaic function can be separately enabled/disabled for BG0-BG3 by
BG0CNT-BG3CNT Registers, as well as for each OBJ0-127 by OBJ attributes in OAM
memory. Also, setting all of the bits below to zero effectively disables the
mosaic function.

  Bit   Expl.
  0-3   BG Mosaic H-Size  (minus 1)
  4-7   BG Mosaic V-Size  (minus 1)
  8-11  OBJ Mosaic H-Size (minus 1)
  12-15 OBJ Mosaic V-Size (minus 1)
  16-31 Not used

Example: When setting H-Size to 5, then pixels 0-5 of each display row are
colorized as pixel 0, pixels 6-11 as pixel 6, pixels 12-17 as pixel 12, and so
on.

Normally, a 'mosaic-pixel' is colorized by the color of the upperleft covered
pixel. In many cases it might be more desireful to use the color of the pixel
in the center of the covered area - this effect may be gained by scrolling the
background (or by adjusting the OBJ position, as far as upper/left rows/columns
of OBJ are transparent).
```
