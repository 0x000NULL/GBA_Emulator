# GBA Cart I/O Port (GPIO)

Source: https://problemkaputt.de/gbatek.htm#gbacartioportgpio

```
4bit General Purpose I/O Port (GPIO) - contained in the ROM-chip

Used by Boktai for RTC and Solar Sensor:

GBA Cart Real-Time Clock (RTC)

GBA Cart Solar Sensor

And by Warioware Twisted for Rumble and Z-Axis Sensor:

GBA Cart Rumble

GBA Cart Gyro Sensor

Might be also used by other games for other purposes, such like other sensors,
or SRAM bank switching, etc.

The I/O registers are mapped to a 6-byte region in the ROM-area at 80000C4h,
the 6-byte region should be zero-filled in the ROM-image. In Boktai, the size
of the zero-filled region is 0E0h bytes - that probably due to an incorrect
definition (the additional bytes do not contain any extra ports, nor mirrors of
the ports in the 6-byte region). Observe that ROM-bus writes are limited to
16bit/32bit access (STRB opcodes are ignored; that, only in DS mode?).

80000C4h - I/O Port Data (selectable W or R/W)

  bit0-3  Data Bits 0..3 (0=Low, 1=High)
  bit4-15 not used (0)

80000C6h - I/O Port Direction (for above Data Port) (selectable W or R/W)

  bit0-3  Direction for Data Port Bits 0..3 (0=In, 1=Out)
  bit4-15 not used (0)

80000C8h - I/O Port Control (selectable W or R/W)

  bit0    Register 80000C4h..80000C8h Control (0=Write-Only, 1=Read/Write)
  bit1-15 not used (0)

In write-only mode, reads return 00h (or possible other data, if the rom
contains non-zero data at that location).

Connection Examples

  GPIO       | Boktai  | Wario
  Bit Pin    | RTC SOL | GYR RBL
  -----------+---------+---------
  0   ROM.1  | SCK CLK | RES -
  1   ROM.2  | SIO RST | CLK -
  2   ROM.21 | CS  -   | DTA -
  3   ROM.22 | -   FLG | -   MOT
  -----------+---------+---------
  IRQ ROM.43 | IRQ -   | -   -

Aside from the I/O Port, the ROM-chip also includes an inverter (used for
inverting the RTC /IRQ signal), and some sort of an (unused) address decoder
output (which appears to be equal or related to A23 signal) (ie. reacting on
ROM A23, or SRAM D7, which share the same pin on GBA slot).
```
