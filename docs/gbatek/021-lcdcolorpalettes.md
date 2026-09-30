# LCD Color Palettes

Source: https://problemkaputt.de/gbatek.htm#lcdcolorpalettes

```
Color Palette RAM

BG and OBJ palettes are using separate memory regions:

  05000000-050001FF - BG Palette RAM (512 bytes, 256 colors)
  05000200-050003FF - OBJ Palette RAM (512 bytes, 256 colors)

Each BG and OBJ palette RAM may be either split into 16 palettes with 16 colors
each, or may be used as a single palette with 256 colors.

Note that some OBJs may access palette RAM in 16 color mode, while other OBJs
may use 256 color mode at the same time. Same for BG0-BG3 layers.

Transparent Colors

Color 0 of all BG and OBJ palettes is transparent. Even though palettes are
described as 16 (256) color palettes, only 15 (255) colors are actually
visible.

Backdrop Color

Color 0 of BG Palette 0 is used as backdrop color. This color is displayed if
an area of the screen is not covered by any non-transparent BG or OBJ dots.

Color Definitions

Each color occupies two bytes (same as for 32768 color BG modes):

  Bit   Expl.
  0-4   Red Intensity   (0-31)
  5-9   Green Intensity (0-31)
  10-14 Blue Intensity  (0-31)
  15    Not used

Intensities

Under normal circumstances (light source/viewing angle), the intensities 0-14
are practically all black, and only intensities 15-31 are resulting in visible
medium..bright colors.

Note: The intensity problem appears in the 8bit CGB "compatibility" mode
either. The original CGB display produced the opposite effect: Intensities 0-14
resulted in dark..medium colors, and intensities 15-31 resulted in bright
colors. Any "medium" colors of CGB games will appear invisible/black on GBA
hardware, and only very bright colors will be visible.
```
