# GBA Cart Real-Time Clock (RTC)

Source: https://problemkaputt.de/gbatek.htm#gbacartrealtimeclockrtc

```
S3511 - 8pin RTC with 3-wire serial bus (used in Boktai)

The RTC chip is (almost) the same as used in NDS consoles:

DS Real-Time Clock (RTC)

The chip is accessed via 4bit I/O port (only 3bits are used for RTC):

GBA Cart I/O Port (GPIO)

Comparision of RTC Registers

  NDS_________GBA_________GBA/Params___
  stat2       control     (1-byte)
  datetime    datetime    (7-byte)
  time        time        (3-byte)
  stat1       force reset (0-byte)
  clkadjust   force irq   (0-byte)
  alarm1/int1 always FFh  (boktai contains code for writing 1-byte to it)
  alarm2      always FFh  (unused)
  free        always FFh  (unused)

Control Register

  Bit Dir Expl.
  0   -   Not used
  1   R/W IRQ duty/hold related?
  2   -   Not used
  3   R/W Per Minute IRQ (30s duty)        (0=Disable, 1=Enable)
  4   -   Not used
  5   R/W Unknown?
  6   R/W 12/24-hour Mode                  (0=12h, 1=24h) (usually 1)
  7   R   Power-Off (auto cleared on read) (0=Normal, 1=Failure, time lost)

Setting after Battery-Shortcut is 82h. Setting after Force-Reset is 00h.

Unused bits seem to be always zero, but might be read-only or write-only?

Datetime and Time Registers

Same as NDS, except AM/PM flag moved from hour.bit6 (NDS) to hour.bit7 (GBA).

Force Reset/Irq Registers

Used to reset all RTC registers (all used registers become 00h, except
day/month which become 01h), or to drag the IRQ output LOW for a short moment.
These registers are strobed by ANY access to them, ie. by both writing to, as
well as reading from these registers.

RTC Games

  Boktai series     ;which/how many titles?
  P-Letter series   ;which/how many titles?
  Rockman EXE 4.5 Real Operation

Pin-Outs / IRQ Signal

The package has identical pin-outs as in NDS, although it is slightly larger
than the miniature chip in the DS.

For whatever reason, the RTC's /IRQ output is passed through an inverter
(contained in the ROM-chip), the inverted signal is then passed to the /IRQ pin
on the cartridge slot. So, IRQ's will be triggered on the "wrong" edge -
possible somehow in relation with detecting cartridge-removal IRQs?
```
