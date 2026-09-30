# LCD I/O BG Scrolling

Source: https://problemkaputt.de/gbatek.htm#lcdiobgscrolling

```
4000010h - BG0HOFS - BG0 X-Offset (W)

4000012h - BG0VOFS - BG0 Y-Offset (W)

  Bit   Expl.
  0-8   Offset (0-511)
  9-15  Not used

Specifies the coordinate of the upperleft first visible dot of BG0 background
layer, ie. used to scroll the BG0 area.

4000014h - BG1HOFS - BG1 X-Offset (W)

4000016h - BG1VOFS - BG1 Y-Offset (W)

Same as above BG0HOFS and BG0VOFS for BG1 respectively.

4000018h - BG2HOFS - BG2 X-Offset (W)

400001Ah - BG2VOFS - BG2 Y-Offset (W)

Same as above BG0HOFS and BG0VOFS for BG2 respectively.

400001Ch - BG3HOFS - BG3 X-Offset (W)

400001Eh - BG3VOFS - BG3 Y-Offset (W)

Same as above BG0HOFS and BG0VOFS for BG3 respectively.

The above BG scrolling registers are exclusively used in Text modes, ie. for
all layers in BG Mode 0, and for the first two layers in BG mode 1.

In other BG modes (Rotation/Scaling and Bitmap modes) above registers are
ignored. Instead, the screen may be scrolled by modifying the BG
Rotation/Scaling Reference Point registers.
```
