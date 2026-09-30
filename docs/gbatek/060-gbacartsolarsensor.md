# GBA Cart Solar Sensor

Source: https://problemkaputt.de/gbatek.htm#gbacartsolarsensor

```
Uses a Photo Diode as Solar Sensor (used in Boktai, allowing to defeat vampires
when the cartridge is exposed to sunlight). The cartridge comes in transparent
case, and it's slightly longer than normal carts, so the sensor reaches out of
the cartridge slot. According to the manual, the sensor works only with
sunlight, but actually it works with any strong light source (eg. a 100 Watt
bulb at 1-2 centimeters distance). The sensor is accessed via 4bit I/O port
(only 3bits used), which is contained in the ROM-chip.

GBA Cart I/O Port (GPIO)

A/D Conversion

The cartridge uses a self-made digital-ramp converter A/D converter, which is
(maybe) better than measuring a capacitor charge-up time, and/or less expensive
than a real ADC-chip:

It contains a 74LV4040 12bit binary counter (clocked by CPU via the I/O port),
of which only the lower 8bit are used, which are passed to a resistor
ladder-type D/A converter, which is generating a linear increasing voltage,
which is passed to a TLV272 voltage comparator, which is passing a signal to
the I/O port when the counter voltage becomes greater than the sensor voltage.

Example Code

  strh  0001h,[80000c8h] ;-enable R/W mode
  strh  0007h,[80000c6h] ;-init I/O direction
  strh  0002h,[80000c4h] ;-reset counter to zero (high=reset) (I/O bit0)
  strh  0000h,[80000c4h] ;-clear reset (low=normal)
  mov   r0,0             ;-initial level
 @@lop:
  strh  0001h,[80000c4h] ;-clock high ;\increase counter      (I/O bit1)
  strh  0000h,[80000c4h] ;-clock low  ;/
  ldrh  r1,[80000c4h]    ;-read port                          (I/O bit3)
  tst   r1,08h           ;\
  addeq r0,1             ; loop until voltage match (exit with r0=00h..FFh),
  tsteq r0,100h          ; or until failure/timeout (exit with r0=100h)
  beq   @@lop            ;/

The results vary depending on the clock rate used. In above example, ensure
that IRQs or DMAs do not interrupt the function. Alternately, use a super-slow
clock rate (eg. like 666Hz used in Boktai) so that additional small IRQ/DMA
delays have little effect on the overall timing. Results should be somewhat:

  E8h  total darkness (including LED light, or daylight on rainy days)
  Dxh  close to a 100 Watt Bulb
  5xh  reaches max level in boktai's solar gauge
  00h  close to a tactical nuclear bomb dropped on your city

The exact values may change from cartridge to cartridge, so it'd be recommened
to include a darkness calibration function, prompting the user to cover the
sensor for a moment (in Boktai, access Options by pressing left/right in title
screen) (alternately, auto-calibration could theoretically memorize the darkest
in-game level ever seen).
```
