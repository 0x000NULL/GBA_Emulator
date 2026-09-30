# GBA Cart Gyro Sensor

Source: https://problemkaputt.de/gbatek.htm#gbacartgyrosensor

```
Warioware Twisted (Z-Axis Gyro Sensor, plus Rumble)

Wario-Type (Z-Axis)

Uses a single-axis sensor, which senses rotation around the Z-axis. The sensor
is connected to an analogue-in, serial-out ADC chip, which is accessed via
lower 3 bits of the GPIO,

GBA Cart I/O Port (GPIO)

The four I/O Lines are connected like so,

  GPIO.Bit0 (W) Start Conversion
  GPIO.Bit1 (W) Serial Clock
  GPIO.Bit2 (R) Serial Data
  GPIO.Bit3 (W) Used for Rumble (not gyro related)

There should be at least <three sequential 32bit ARM opcodes executed in WS0
region> between the STRH opcodes which toggle the CLK signal. Wario uses
WAITCNT=45B7h (SRAM=8clks, WS0/WS1/WS2=3,1clks, Prefetch=On, PHI=Off).

The data stream consists of: 4 dummy bits (usually zero), followed by 12 data
bits, followed by endless unused bits (usually zero).

 read_gyro:
  mov  r1,8000000h      ;-cartridge base address
  mov  r0,01h           ;\enable R/W access
  strh r0,[r1,0c8h]     ;/
  mov  r0,0bh           ;\init direction (gpio2=input, others=output)
  strh r0,[r1,0c6h]     ;/
  ldrh r2,[r1,0c4h]     ;-get current state (for keeping gpio3=rumble)
  orr  r2,3                     ;\
  strh r2,[r1,0c4h] ;gpio0=1    ; start ADC conversion
  bic  r2,1                     ;
  strh r2,[r1,0c4h] ;gpio0=0    ;/
  mov  r0,00010000h ;stop-bit           ;\
  bic  r2,2                             ;
 @@lop:                                 ;
  ldrh r3,[r1,0c4h] ;get gpio2=data     ; read 16 bits
  strh r2,[r1,0c4h] ;gpio1=0=clk=low    ; (4 dummy bits, plus 12 data bits)
  movs r3,r3,lsr 3  ;gpio2 to cy=data   ;
  adcs r0,r0,r0     ;merge data, cy=done;
  orr  r3,r2,2      ;set bit1 and delay ;
  strh r3,[r1,0c4h] ;gpio1=1=clk=high   ;
  bcc  @@lop                            ;/
  bic  r0,0f000h                 ;-strip upper 4 dummy bits (isolate 12bit adc)
  bx   lr

Example values (may vary on different carts, battery charge, temperature, etc):

  354h  rotated in anti-clockwise direction (shock-speed)
  64Dh  rotated in anti-clockwise direction (normal fast)
  6A3h  rotated in anti-clockwise direction (slow)
  6C0h  no rotation                         (stopped)
  6DAh  rotation in clockwise direction     (slow)
  73Ah  rotation in clockwise direction     (normal fast)
  9E3h  rotation in clockwise direction     (shock-speed)

For detection, values 000h and FFFh would indicate that there's no sensor.

The Z-axis always points into same direction; no matter of frontloading or
backloading cartridge slots.

Thanks to Momo Vampire for contributing a Wario cartridge.

X/Y/Z-Axes

X-Axis and Y-Axis are meant to be following the screens X and Y coordinates, so
the Z-Axis would point into the screens depth direction.

DSi Cameras

DSi consoles can mis-use the built-in cameras as Gyro sensor (as done by the
System Flaw DSi game).
```
