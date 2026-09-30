# GBA Cart Rumble

Source: https://problemkaputt.de/gbatek.htm#gbacartrumble

```
Warioware Twisted (Rumble, plus Z-Axis Gyro Sensor)

Drill Dozer (Rumble only) <-- and ALSO supports Gameboy Player rumble?

GBA Rumble Carts are containing a small motor, which is causing some vibration
when/while it is switched on (that, unlike DS Rumble, which must be repeatedly
toggled on/off).

In Warioware Twisted, rumble is controlled via GPIO.Bit3 (Data 0=Low=Off,
1=High=On) (and Direction 1=Output), the other GPIO Bits are used for the gyro
sensor.

GBA Cart I/O Port (GPIO)

Note: GPIO3 is connected to an external pulldown resistor (so the HighZ level
gets dragged to Low=Off when direction is set to Input).

Unknown if Drill Dozer is controlled via GPIO.Bit3, too?

DS Rumble Pak

Additionally, there's a Rumble Pak for the NDS, which connects to the GBA slot,
so it can be used also for GBA games (provided that the game doesn't require
the GBA slot, eg. GBA multiboot games).

DS Cart Rumble Pak

Gamecube Rumble

Moreover, GBA games that are running on a Gameboy Player are having access to
the Rumble function of Gamecube joypads.

GBA Gameboy Player
```
