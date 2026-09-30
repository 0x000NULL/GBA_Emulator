# SIO General-Purpose Mode

Source: https://problemkaputt.de/gbatek.htm#siogeneralpurposemode

```
In this mode, the SIO is 'misused' as a 4bit bi-directional parallel port, each
of the SI,SO,SC,SD pins may be directly controlled, each can be separately
declared as input (with internal pull-up) or as output signal.

4000134h - RCNT (R) - SIO Mode, usage in GENERAL-PURPOSE Mode (R/W)

Interrupts can be requested when SI changes from HIGH to LOW, as General
Purpose mode does not require a serial shift clock, this interrupt may be
produced even when the GBA is in Stop (low power standby) state.

  Bit   Expl.
  0     SC Data Bit         (0=Low, 1=High)
  1     SD Data Bit         (0=Low, 1=High)
  2     SI Data Bit         (0=Low, 1=High)
  3     SO Data Bit         (0=Low, 1=High)
  4     SC Direction        (0=Input, 1=Output)
  5     SD Direction        (0=Input, 1=Output)
  6     SI Direction        (0=Input, 1=Output, but see below)
  7     SO Direction        (0=Input, 1=Output)
  8     SI Interrupt Enable (0=Disable, 1=Enable)
  9-13  Not used
  14    Must be "0" for General-Purpose Mode
  15    Must be "1" for General-Purpose or JOYBUS Mode

SI should be always used as Input to avoid problems with other hardware which
does not expect data to be output there.

4000128h - SIOCNT - SIO Control, not used in GENERAL-PURPOSE Mode

This register is not used in general purpose mode. That is, the separate bits
of SIOCNT still exist and are read- and/or write-able in the same manner as for
Normal, Multiplay, or UART mode (depending on SIOCNT Bit 12,13), but are having
no effect on data being output to the link port.
```
