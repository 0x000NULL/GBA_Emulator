# GBA Cart Tilt Sensor

Source: https://problemkaputt.de/gbatek.htm#gbacarttiltsensor

```
Yoshi's Universal Gravitation / Yoshi Topsy Turvy (X/Y-Axis)

Koro Koro Puzzle (probably same as Yoshi, X/Y-Axis, too) (?)

Yoshi-Type (X/Y-Axis)

All of the registers are one byte wide, mapped into the top "half" of the SRAM
memory range.

  E008000h (W) Write 55h to start sampling
  E008100h (W) Write AAh to start sampling
  E008200h (R) Lower 8 bits of X axis
  E008300h (R) Upper 4 bits of X axis, and Bit7: ADC Status (0=Busy, 1=Ready)
  E008400h (R) Lower 8 bits of Y axis
  E008500h (R) Upper 4 bits of Y axis

You must set SRAM wait control to 8 clocks to access it correctly.

You must also set the cartridge PHI terminal to 4 MHz to make it work.

Sampling routine (typically executed once a frame during VBlank):

  wait until [E008300h].Bit7=1 or until timeout ;wait ready
  x = ([E008300h] AND 0Fh)*100h + [E008200h]    ;get x
  y = ([E008500h] AND 0Fh)*100h + [E008400h]    ;get y
  [E008000h]=55h, [E008100h]=AAh                ;start next conversion

Example values (may vary on different carts and on temperature, etc):

  X ranged between 0x2AF to 0x477, center at 0x392.    Huh?
  Y ranged between 0x2C3 to 0x480, center at 0x3A0.    Huh?

Thanks to Flubba for Yoshi-Type information.

Unknown if the Yoshi-Type sensors are sensing rotation, or orientation, or
motion, or something else? In case of rotation, rotation around X-axis would
result in motion in Y-direction, so not too sure whether X and Y have which
meaning?

Most probably, the sensors are measuring (both) static acceleration (gravity),
and dynamic acceleration (eg. shaking the device left/right).

The X/Y values are likely to be mirrored depending on using a back-loading
cartridge slot (original GBA), or front-loading cartridge slot (newer GBA SP,
and NDS, and NDS-Lite).
```
