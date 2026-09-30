# LCD I/O Color Special Effects

Source: https://problemkaputt.de/gbatek.htm#lcdiocolorspecialeffects

```
Two types of Special Effects are supported: Alpha Blending (Semi-Transparency)
allows to combine colors of two selected surfaces. Brightness Increase/Decrease
adjust the brightness of the selected surface.

4000050h - BLDCNT - Color Special Effects Selection (R/W)

  Bit   Expl.
  0     BG0 1st Target Pixel (Background 0)
  1     BG1 1st Target Pixel (Background 1)
  2     BG2 1st Target Pixel (Background 2)
  3     BG3 1st Target Pixel (Background 3)
  4     OBJ 1st Target Pixel (Top-most OBJ pixel)
  5     BD  1st Target Pixel (Backdrop)
  6-7   Color Special Effect (0-3, see below)
         0 = None                (Special effects disabled)
         1 = Alpha Blending      (1st+2nd Target mixed)
         2 = Brightness Increase (1st Target becomes whiter)
         3 = Brightness Decrease (1st Target becomes blacker)
  8     BG0 2nd Target Pixel (Background 0)
  9     BG1 2nd Target Pixel (Background 1)
  10    BG2 2nd Target Pixel (Background 2)
  11    BG3 2nd Target Pixel (Background 3)
  12    OBJ 2nd Target Pixel (Top-most OBJ pixel)
  13    BD  2nd Target Pixel (Backdrop)
  14-15 Not used

Selects the 1st Target layer(s) for special effects. For Alpha
Blending/Semi-Transparency, it does also select the 2nd Target layer(s), which
should have next lower display priority as the 1st Target.

However, any combinations are possible, including that all layers may be
selected as both 1st+2nd target, in that case the top-most pixel will be used
as 1st target, and the next lower pixel as 2nd target.

4000052h - BLDALPHA - Alpha Blending Coefficients (R/W) (not W)

Used for Color Special Effects Mode 1, and for Semi-Transparent OBJs.

  Bit   Expl.
  0-4   EVA Coefficient (1st Target) (0..16 = 0/16..16/16, 17..31=16/16)
  5-7   Not used
  8-12  EVB Coefficient (2nd Target) (0..16 = 0/16..16/16, 17..31=16/16)
  13-15 Not used

For this effect, the top-most non-transparent pixel must be selected as 1st
Target, and the next-lower non-transparent pixel must be selected as 2nd
Target, if so - and only if so, then color intensities of 1st and 2nd Target
are mixed together by using the parameters in BLDALPHA register, for each pixel
each R, G, B intensities are calculated separately:

  I = MIN ( 31, I1st*EVA + I2nd*EVB )

Otherwise - for example, if only one target exists, or if a non-transparent
non-2nd-target pixel is moved between the two targets, or if 2nd target has
higher display priority than 1st target - then only the top-most pixel is
displayed (at normal intensity, regardless of BLDALPHA).

4000054h - BLDY - Brightness (Fade-In/Out) Coefficient (W) (not R/W)

Used for Color Special Effects Modes 2 and 3.

  Bit   Expl.
  0-4   EVY Coefficient (Brightness) (0..16 = 0/16..16/16, 17..31=16/16)
  5-31  Not used

For each pixel each R, G, B intensities are calculated separately:

  I = I1st + (31-I1st)*EVY   ;For Brightness Increase
  I = I1st - (I1st)*EVY      ;For Brightness Decrease

The color intensities of any selected 1st target surface(s) are increased or
decreased by using the parameter in BLDY register.

Semi-Transparent OBJs

OBJs that are defined as 'Semi-Transparent' in OAM memory are always selected
as 1st Target (regardless of BLDCNT Bit 4), and are always using Alpha Blending
mode (regardless of BLDCNT Bit 6-7).

The BLDCNT register may be used to perform Brightness effects on the OBJ
(and/or other BG/BD layers). However, if a semi-transparent OBJ pixel does
overlap a 2nd target pixel, then semi-transparency becomes priority, and the
brightness effect will not take place (neither on 1st, nor 2nd target).

The OBJ Layer

Before special effects are applied, the display controller computes the OBJ
priority ordering, and isolates the top-most OBJ pixel. In result, only the
top-most OBJ pixel is recursed at the time when processing special effects. Ie.
alpha blending and semi-transparency can be used for OBJ-to-BG or BG-to-OBJ ,
but not for OBJ-to-OBJ.
```
