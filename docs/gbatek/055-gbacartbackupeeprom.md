# GBA Cart Backup EEPROM

Source: https://problemkaputt.de/gbatek.htm#gbacartbackupeeprom

```
9853 - EEPROM 512 Bytes (0200h) (4Kbit) (eg. used by Super Mario Advance)

9854 - EEPROM 8 KBytes (2000h) (64Kbit) (eg. used by Boktai)

Lifetime: 100,000 writes per address

Addressing and Waitstates

The eeprom is connected to Bit0 of the data bus, and to the upper 1 bit (or
upper 17 bits in case of large 32MB ROM) of the cartridge ROM address bus,
communication with the chip takes place serially.

The eeprom must be used with 8 waitstates (set WAITCNT=X3XXh; 8,8 clks in WS2
area), the eeprom can be then addressed at DFFFF00h..DFFFFFFh.

Respectively, with eeprom, ROM is restricted to 8000000h-9FFFeFFh (max.
1FFFF00h bytes = 32MB minus 256 bytes). On carts with 16MB or smaller ROM,
eeprom can be alternately accessed anywhere at D000000h-DFFFFFFh.

Data and Address Width

Data can be read from (or written to) the EEPROM in units of 64bits (8 bytes).
Writing automatically erases the old 64bits of data. Addressing works in units
of 64bits respectively, that is, for 512 Bytes EEPROMS: an address range of
0-3Fh, 6bit bus width; and for 8KByte EEPROMs: a range of 0-3FFh, 14bit bus
width (only the lower 10 address bits are used, upper 4 bits should be zero).

Set Address (For Reading)

Prepare the following bitstream in memory:

  2 bits "11" (Read Request)
  n bits eeprom address (MSB first, 6 or 14 bits, depending on EEPROM)
  1 bit "0"

Then transfer the stream to eeprom by using DMA.

Read Data

Read a stream of 68 bits from EEPROM by using DMA,

then decipher the received data as follows:

  4 bits - ignore these
 64 bits - data (conventionally MSB first)

Write Data to Address

Prepare the following bitstream in memory, then transfer the stream to eeprom
by using DMA, it'll take ca. 108368 clock cycles (ca. 6.5ms) until the old data
is erased and new data is programmed.

  2 bits "10" (Write Request)
  n bits eeprom address (MSB first, 6 or 14 bits, depending on EEPROM)
 64 bits data (conventionally MSB first)
  1 bit "0"

After the DMA, keep reading from the chip, by normal LDRH [DFFFF00h], until Bit
0 of the returned data becomes "1" (Ready). To prevent your program from
locking up in case of malfunction, generate a timeout if the chip does not
reply after 10ms or longer.

Using DMA

Transferring a bitstreams to/from the EEPROM must be done via DMA3 (manual
transfers via LDRH/STRH won't work; probably because they don't keep /CS=LOW
and A23=HIGH throughout the transfer).

For using DMA, a buffer in memory must be used (that buffer would be typically
allocated temporarily on stack, one halfword for each bit, bit1-15 of the
halfwords are don't care, only bit0 is used).

The buffer must be transfered as a whole to/from EEPROM by using DMA3 (DMA0-2
can't access external memory), use 16bit transfer mode, both source and
destination address incrementing (ie. DMA3CNT=80000000h+length).

DMA channels of higher priority should be disabled during the transfer (ie.
H/V-Blank or Sound FIFO DMAs). And, of course any interrupts that might mess
with DMA registers should be disabled.

Pin-Outs

The EEPROM chips are having only 8 pins, these are connected, Pin 1..8, to
ROMCS, RD, WR, AD0, GND, GND, A23, VDD of the GamePak bus. Carts with 32MB ROM
must have A7..A22 logically ANDed with A23.

The chips seem to be some custom design, the pinout, transfer protocol and
8-byte pages don't resemble any standard serial EEPROMs.

The chips have no other markings than 9853 or 9854 (plus some datecode),
without any manufacturer info (https://gbhwdb.gekkio.fi/
 declares them as Rohm chips).

Notes

There seems to be no autodection mechanism, so that a hardcoded bus width must
be used.
```
