# LCD I/O Display Control

Source: https://problemkaputt.de/gbatek.htm#lcdiodisplaycontrol

```
4000000h - DISPCNT - LCD Control (Read/Write)

  Bit   Expl.
  0-2   BG Mode                (0-5=Video Mode 0-5, 6-7=Prohibited)
  3     Reserved / CGB Mode    (0=GBA, 1=CGB; can be set only by BIOS opcodes)
  4     Display Frame Select   (0-1=Frame 0-1) (for BG Modes 4,5 only)
  5     H-Blank Interval Free  (1=Allow access to OAM during H-Blank)
  6     OBJ Character VRAM Mapping (0=Two dimensional, 1=One dimensional)
  7     Forced Blank           (1=Allow FAST access to VRAM,Palette,OAM)
  8     Screen Display BG0  (0=Off, 1=On)
  9     Screen Display BG1  (0=Off, 1=On)
  10    Screen Display BG2  (0=Off, 1=On)
  11    Screen Display BG3  (0=Off, 1=On)
  12    Screen Display OBJ  (0=Off, 1=On)
  13    Window 0 Display Flag   (0=Off, 1=On)
  14    Window 1 Display Flag   (0=Off, 1=On)
  15    OBJ Window Display Flag (0=Off, 1=On)

The table summarizes the facilities of the separate BG modes (video modes).

  Mode  Rot/Scal Layers Size               Tiles Colors       Features
  0     No       0123   256x256..512x515   1024  16/16..256/1 SFMABP
  1     Mixed    012-   (BG0,BG1 as above Mode 0, BG2 as below Mode 2)
  2     Yes      --23   128x128..1024x1024 256   256/1        S-MABP
  3     Yes      --2-   240x160            1     32768        --MABP
  4     Yes      --2-   240x160            2     256/1        --MABP
  5     Yes      --2-   160x128            2     32768        --MABP

Features: S)crolling, F)lip, M)osaic, A)lphaBlending, B)rightness, P)riority.

BG Modes 0-2 are Tile/Map-based. BG Modes 3-5 are Bitmap-based, in these modes
1 or 2 Frames (ie. bitmaps, or 'full screen tiles') exists, if two frames
exist, either one can be displayed, and the other one can be redrawn in
background.

Blanking Bits

Setting Forced Blank (Bit 7) causes the video controller to display white
lines, and all VRAM, Palette RAM, and OAM may be accessed.

"When the internal HV synchronous counter cancels a forced blank during a
display period, the display begins from the beginning, following the display of
two vertical lines." What ?

Setting H-Blank Interval Free (Bit 5) allows to access OAM during H-Blank time
- using this feature reduces the number of sprites that can be displayed per
line.

Display Enable Bits

By default, BG0-3 and OBJ Display Flags (Bit 8-12) are used to enable/disable
BGs and OBJ. When enabling Window 0 and/or 1 (Bit 13-14), color special effects
may be used, and BG0-3 and OBJ are controlled by the window(s).

Frame Selection

In BG Modes 4 and 5 (Bitmap modes), either one of the two bitmaps/frames may be
displayed (Bit 4), allowing the user to update the other (invisible) frame in
background. In BG Mode 3, only one frame exists.

In BG Modes 0-2 (Tile/Map based modes), a similar effect may be gained by
altering the base address(es) of BG Map and/or BG Character data.

4000002h - Undocumented - Green Swap (R/W)

Normally, red green blue intensities for a group of two pixels is output as
BGRbgr (uppercase for left pixel at even xloc, lowercase for right pixel at odd
xloc). When the Green Swap bit is set, each pixel group is output as BgRbGr
(ie. green intensity of each two pixels exchanged).

  Bit   Expl.
  0     Green Swap  (0=Normal, 1=Swap)
  1-15  Not used

This feature appears to be applied to the final picture (ie. after mixing the
separate BG and OBJ layers). Eventually intended for other display types (with
other pin-outs). With normal GBA hardware it is just producing an interesting
dirt effect.

The NDS DISPCNT registers are 32bit (4000000h..4000003h), so Green Swap doesn't
exist in NDS mode, however, the NDS does support Green Swap in GBA mode.
```
