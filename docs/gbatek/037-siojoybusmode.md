# SIO JOY BUS Mode

Source: https://problemkaputt.de/gbatek.htm#siojoybusmode

```
This communication mode uses Nintendo's standardized JOY Bus protocol. When
using this communication mode, the GBA is always operated as SLAVE!

The JOY BUS mode is intended for communicating with Nintendo Gamecube (DOL)
consoles, where the GBA would somehow act as peripheral for the Gamecube
(unknown if/which Gamecube games are supporting that feature).

In this mode, SI and SO pins are data lines (apparently synchronized by
Start/Stop bits?), SC and SD are set to low (including during active
transfer?), the transfer rate is unknown?

4000134h - RCNT (R) - Mode Selection, in JOY BUS mode (R/W)

  Bit   Expl.
  0-3   Undocumented (current SC,SD,SI,SO state, as for General Purpose mode)
  4-8   Not used     (Should be 0, bits are read/write-able though)
  9-13  Not used     (Always 0, read only)
  14    Must be "1" for JOY BUS Mode
  15    Must be "1" for JOY BUS Mode

4000128h - SIOCNT - SIO Control, not used in JOY BUS Mode

This register is not used in JOY BUS mode.

4000140h - JOYCNT - JOY BUS Control Register (R/W)

  Bit   Expl.
  0     Device Reset Flag     (Command FFh)          (Read/Acknowledge)
  1     Receive Complete Flag (Command 14h or 15h?)  (Read/Acknowledge)
  2     Send Complete Flag    (Command 15h or 14h?)  (Read/Acknowledge)
  3-5   Not used
  6     IRQ when receiving a Device Reset Command  (0=Disable, 1=Enable)
  7-31  Not used

Bit 0-2 are working much like the bits in the IF register: Write a "1" bit to
reset (acknowledge) the respective bit.

UNCLEAR: Interrupts can be requested for Send/Receive commands also?

4000150h - JOY_RECV_L - Receive Data Register low (R/W)

4000152h - JOY_RECV_H - Receive Data Register high (R/W)

4000154h - JOY_TRANS_L - Send Data Register low (R/W)

4000156h - JOY_TRANS_H - Send Data Register high (R/W)

Send/receive data registers.

4000158h - JOYSTAT - Receive Status Register (R/W)

  Bit   Expl.
  0     Not used
  1     Receive Status Flag   (0=Remote Side is/was receiving) (Read Only?)
  2     Not used
  3     Send Status Flag      (1=Remote Side is/was sending)   (Read Only?)
  4-5   General Purpose Flag  (Not assigned, may be used for whatever purpose)
  6-31  Not used

Bit 1 is automatically set when writing to local JOY_TRANS.

Bit 3 is automatically reset when reading from local JOY_RECV.

Below are the four possible commands which can be received by the GBA. Note
that the GBA (slave) cannot send any commands itself, all it can do is to read
incoming data, and to provide 'reply' data which may (or may not) be read out
by the master unit.

Command FFh - Device Reset

  Receive FFh (Command)
  Send    00h (GBA Type number LSB)
  Send    04h (GBA Type number MSB)
  Send    XXh (lower 8bits of JOYSTAT register)

Command 00h - Type/Status Data Request

  Receive 00h (Command)
  Send    00h (GBA Type number LSB)
  Send    04h (GBA Type number MSB)
  Send    XXh (lower 8bits of JOYSTAT register)

Command 15h - GBA Data Write (to GBA)

  Receive 15h (Command)
  Receive XXh (Lower 8bits of JOY_RECV_L)
  Receive XXh (Upper 8bits of JOY_RECV_L)
  Receive XXh (Lower 8bits of JOY_RECV_H)
  Receive XXh (Upper 8bits of JOY_RECV_H)
  Send    XXh (lower 8bits of JOYSTAT register)

Command 14h - GBA Data Read (from GBA)

  Receive 14h (Command)
  Send    XXh (Lower 8bits of JOY_TRANS_L)
  Send    XXh (Upper 8bits of JOY_TRANS_L)
  Send    XXh (Lower 8bits of JOY_TRANS_H)
  Send    XXh (Upper 8bits of JOY_TRANS_H)
  Send    XXh (lower 8bits of JOYSTAT register)

See also

https://hitmen.c02.at/files/yagcd/yagcd/chap9.html
```
