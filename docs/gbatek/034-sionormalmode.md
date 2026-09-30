# SIO Normal Mode

Source: https://problemkaputt.de/gbatek.htm#sionormalmode

```
This mode is used to communicate between two units.

Transfer rates of 256Kbit/s or 2Mbit/s can be selected, however, the fast
2Mbit/s is intended ONLY for special hardware expansions that are DIRECTLY
connected to the GBA link port (ie. without a cable being located between the
GBA and expansion hardware). In normal cases, always use 256Kbit/s transfer
rate which provides stable results.

Transfer lengths of 8bit or 32bit may be used, the 8bit mode is the same as for
older DMG/CGB gameboys, however, the voltages for "GBA cartridges in GBAs" are
different as for "DMG/CGB cartridges in DMG/CGB/GBAs", ie. it is not possible
to communicate between DMG/CGB games and GBA games.

4000134h - RCNT (R) - Mode Selection, in Normal/Multiplayer/UART modes (R/W)

  Bit   Expl.
  0-3   Undocumented (current SC,SD,SI,SO state, as for General Purpose mode)
  4-8   Not used     (Should be 0, bits are read/write-able though)
  9-13  Not used     (Always 0, read only)
  14    Not used     (Should be 0, bit is read/write-able though)
  15    Must be zero (0) for Normal/Multiplayer/UART modes

4000128h - SIOCNT - SIO Control, usage in NORMAL Mode (R/W)

  Bit   Expl.
  0     Shift Clock (SC)        (0=External, 1=Internal)
  1     Internal Shift Clock    (0=256KHz, 1=2MHz)
  2     SI State (opponents SO) (0=Low, 1=High/None) --- (Read Only)
  3     SO during inactivity    (0=Low, 1=High) (applied ONLY when Bit7=0)
  4-6   Not used                (Read only, always 0 ?)
  7     Start Bit               (0=Inactive/Ready, 1=Start/Active)
  8-11  Not used                (R/W, should be 0)
  12    Transfer Length         (0=8bit, 1=32bit)
  13    Must be "0" for Normal Mode
  14    IRQ Enable              (0=Disable, 1=Want IRQ upon completion)
  15    Not used                (Read only, always 0)

The Start bit is automatically reset when the transfer completes, ie. when all
8 or 32 bits are transferred, at that time an IRQ may be generated.

400012Ah - SIODATA8 - SIO Normal Communication 8bit Data (R/W)

For 8bit normal mode. Contains 8bit data (only lower 8bit are used). Outgoing
data should be written to this register before starting the transfer. During
transfer, transmitted bits are shifted-out (MSB first), and received bits are
shifted-in simultaneously. Upon transfer completion, the register contains the
received 8bit value.

4000120h - SIODATA32_L - SIO Normal Communication lower 16bit data (R/W)

4000122h - SIODATA32_H - SIO Normal Communication upper 16bit data (R/W)

Same as above SIODATA8, for 32bit normal transfer mode respectively.

SIOCNT/RCNT must be set to 32bit normal mode <before> writing to
SIODATA32.

Initialization

First, initialize RCNT register. Second, set mode/clock bits in SIOCNT with
startbit cleared. For master: select internal clock, and (in most cases)
specify 256KHz as transfer rate. For slave: select external clock, the local
transfer rate selection is then ignored, as the transfer rate is supplied by
the remote GBA (or other computer, which might supply custom transfer rates).

Third, set the startbit in SIOCNT with mode/clock bits unchanged.

Recommended Communication Procedure for SLAVE unit (external clock)

- Initialize data which is to be sent to master.

- Set Start flag.

- Set SO to LOW to indicate that master may start now.

- Wait for IRQ (or for Start bit to become zero). (Check timeout here!)

- Set SO to HIGH to indicate that we are not ready.

- Process received data.

- Repeat procedure if more data is to be transferred.

(or is so=high done automatically? would be fine - more stable - otherwise
master may still need delay)

Recommended Communication Procedure for SLAVE unit (external clock)

- Initialize data which is to be sent to master.

- Set Start=0 and SO=0 (SO=LOW indicates that slave is (almost) ready).

- Set Start=1 and SO=1 (SO=HIGH indicates not ready, applied after transfer).

  (Expl. Old SO=LOW kept output until 1st clock bit received).
  (Expl. New SO=HIGH is automatically output at transfer completion).

- Set SO to LOW to indicate that master may start now.

- Wait for IRQ (or for Start bit to become zero). (Check timeout here!)

- Process received data.

- Repeat procedure if more data is to be transferred.

Recommended Communication Procedure for MASTER unit (internal clock)

- Initialize data which is to be sent to slave.

- Wait for SI to become LOW (slave ready). (Check timeout here!)

- Set Start flag.

- Wait for IRQ (or for Start bit to become zero).

- Process received data.

- Repeat procedure if more data is to be transferred.

Cable Protocol

During inactive transfer, the shift clock (SC) is high. The transmit (SO) and
receive (SI) data lines may be manually controlled as described above.

When master sends SC=LOW, each master and slave must output the next outgoing
data bit to SO. When master sends SC=HIGH, each master and slave must read out
the opponents data bit from SI. This is repeated for each of the 8 or 32 bits,
and when completed SC will be kept high again.

Transfer Rates

Either 256KHz or 2MHz rates can be selected for SC, so max 32KBytes (256Kbit)
or 128KBytes (2Mbit) can be transferred per second. However, the software must
process each 8bit or 32bit of transmitted data separately, so the actual
transfer rate will be reduced by the time spent on handling each data unit.

Only 256KHz provides stable results in most cases (such like when linking
between two GBAs). The 2MHz rate is intended for special expansion hardware
(with very short wires) only.

Using Normal mode for One-Way Multiplayer communication

When using normal mode with multiplay-cables, data isn't exchanged between
first and second GBA as usually. Instead, data is shifted from first to last
GBA (the first GBA receives zero, because master SI is shortcut to GND).

This behaviour may be used for fast ONE-WAY data transfer from master to all
other GBAs. For example (3 GBAs linked):

  Step         Sender      1st Recipient   2nd Recipient
  Transfer 1:  DATA #0 --> UNDEF      -->  UNDEF     -->
  Transfer 2:  DATA #1 --> DATA #0    -->  UNDEF     -->
  Transfer 3:  DATA #2 --> DATA #1    -->  DATA #0   -->
  Transfer 4:  DATA #3 --> DATA #2    -->  DATA #1   -->

The recipients should not output any own data, instead they should forward the
previously received data to the next recipient during next transfer (just keep
the incoming data unmodified in the data register).

Due to the delayed forwarding, 2nd recipient should ignore the first incoming
data. After the last transfer, the sender must send one (or more) dummy data
unit(s), so that the last data is forwarded to the 2nd (or further)
recipient(s).
```
