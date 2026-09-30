# LCD I/O Interrupts and Status

Source: https://problemkaputt.de/gbatek.htm#lcdiointerruptsandstatus

```
4000004h - DISPSTAT - General LCD Status (Read/Write)

Display status and Interrupt control. The H-Blank conditions are generated once
per scanline, including for the 'hidden' scanlines during V-Blank.

  Bit   Expl.
  0     V-Blank flag   (Read only) (1=VBlank) (set in line 160..226; not 227)
  1     H-Blank flag   (Read only) (1=HBlank) (toggled in all lines, 0..227)
  2     V-Counter flag (Read only) (1=Match)  (set in selected line)     (R)
  3     V-Blank IRQ Enable         (1=Enable)                          (R/W)
  4     H-Blank IRQ Enable         (1=Enable)                          (R/W)
  5     V-Counter IRQ Enable       (1=Enable)                          (R/W)
  6     Not used (0) / DSi: LCD Initialization Ready (0=Busy, 1=Ready)   (R)
  7     Not used (0) / NDS: MSB of V-Vcount Setting (LYC.Bit8) (0..262)(R/W)
  8-15  V-Count Setting (LYC)      (0..227)                            (R/W)

The V-Count-Setting value is much the same as LYC of older gameboys, when its
value is identical to the content of the VCOUNT register then the V-Counter
flag is set (Bit 2), and (if enabled in Bit 5) an interrupt is requested.

Although the drawing time is only 960 cycles (240*4), the H-Blank flag is "0"
for a total of 1006 cycles.

4000006h - VCOUNT - Vertical Counter (Read only)

Indicates the currently drawn scanline, values in range from 160..227 indicate
'hidden' scanlines within VBlank area.

  Bit   Expl.
  0-7   Current Scanline (LY)      (0..227)                              (R)
  8     Not used (0) / NDS: MSB of Current Scanline (LY.Bit8) (0..262)   (R)
  9-15  Not Used (0)

Note: This is much the same than the 'LY' register of older gameboys.
```
