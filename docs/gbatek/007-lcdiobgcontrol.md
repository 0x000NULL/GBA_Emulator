# LCD I/O BG Control

Source: https://problemkaputt.de/gbatek.htm#lcdiobgcontrol

```
4000008h - BG0CNT - BG0 Control (R/W) (BG Modes 0,1 only)

400000Ah - BG1CNT - BG1 Control (R/W) (BG Modes 0,1 only)

400000Ch - BG2CNT - BG2 Control (R/W) (BG Modes 0,1,2 only)

400000Eh - BG3CNT - BG3 Control (R/W) (BG Modes 0,2 only)

  Bit   Expl.
  0-1   BG Priority           (0-3, 0=Highest)
  2-3   Character Base Block  (0-3, in units of 16 KBytes) (=BG Tile Data)
  4-5   Not used (must be zero) (except in NDS mode: MSBs of char base)
  6     Mosaic                (0=Disable, 1=Enable)
  7     Colors/Palettes       (0=16/16, 1=256/1)
  8-12  Screen Base Block     (0-31, in units of 2 KBytes) (=BG Map Data)
  13    BG0/BG1: Not used (except in NDS mode: Ext Palette Slot for BG0/BG1)
  13    BG2/BG3: Display Area Overflow (0=Transparent, 1=Wraparound)
  14-15 Screen Size (0-3)

Internal Screen Size (dots) and size of BG Map (bytes):

  Value  Text Mode      Rotation/Scaling Mode
  0      256x256 (2K)   128x128   (256 bytes)
  1      512x256 (4K)   256x256   (1K)
  2      256x512 (4K)   512x512   (4K)
  3      512x512 (8K)   1024x1024 (16K)

In case that some or all BGs are set to same priority then BG0 is having the
highest, and BG3 the lowest priority.

In 'Text Modes', the screen size is organized as follows: The screen consists
of one or more 256x256 pixel (32x32 tiles) areas. When Size=0: only 1 area
(SC0), when Size=1 or Size=2: two areas (SC0,SC1 either horizontally or
vertically arranged next to each other), when Size=3: four areas (SC0,SC1 in
upper row, SC2,SC3 in lower row). Whereas SC0 is defined by the normal BG Map
base address (Bit 8-12 of BGxCNT), SC1 uses same address +2K, SC2 address +4K,
SC3 address +6K. When the screen is scrolled it'll always wraparound.

In 'Rotation/Scaling Modes', the screen size is organized as follows, only one
area (SC0) of variable size 128x128..1024x1024 pixels (16x16..128x128 tiles)
exists. When the screen is rotated/scaled (or scrolled?) so that the LCD
viewport reaches outside of the background/screen area, then BG may be either
displayed as transparent or wraparound (Bit 13 of BGxCNT).
```
