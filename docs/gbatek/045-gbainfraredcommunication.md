# GBA Infrared Communication

Source: https://problemkaputt.de/gbatek.htm#gbainfraredcommunication

```
Early GBA prototypes have been intended to include a built-in IR port for
sending and receiving IR signals. Among others, this port could have been used
to communicate with other GBAs, or older CGB models, or TV Remote Controls,
etc.

[ THE INFRARED COMMUNICATION FEATURE IS -NOT- SUPPORTED ANYMORE ]

Anyways, the prototype specifications have been as shown below...

Keep in mind that the IR signal may be interrupted by whatever objects moved
between sender and receiver - the IR port isn't recommended for programs that
require realtime data exchange (such like action games).

4000136h - IR - Infrared Register (R/W)

  Bit   Expl.
  0     Transmission Data  (0=LED Off, 1=LED On)
  1     READ Enable        (0=Disable, 1=Enable)
  2     Reception Data     (0=None, 1=Signal received) (Read only)
  3     AMP Operation      (0=Off, 1=On)
  4     IRQ Enable Flag    (0=Disable, 1=Enable)
  5-15  Not used

When IRQ is enabled, an interrupt is requested if the incoming signal was
0.119us Off (2 cycles), followed by 0.536us On (9 cycles) - minimum timing
periods each.

Transmission Notes

When transmitting an IR signal, note that it'd be not a good idea to keep the
LED turned On for a very long period (such like sending a 1 second
synchronization pulse). The recipient's circuit would treat such a long signal
as "normal IR pollution which is in the air" after a while, and thus ignore the
signal.

Reception Notes

Received data is internally latched. Latched data may be read out by setting
both READ and AMP bits.

Note: Provided that you don't want to receive your own IR signal, be sure to
set Bit 0 to zero before attempting to receive data.

Power-consumption

After using the IR port, be sure to reset the register to zero in order to
reduce battery power consumption.
```
