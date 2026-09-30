# GBA Gameboy Player

Source: https://problemkaputt.de/gbatek.htm#gbagameboyplayer

```
The Gameboy Player is an "adapter" for the Gamecube console. It's basicly is a
GBA in a black box without LCD screen and without buttons, connected to an
expansion port at the bottom of the Gamecube. The Gamecube is then capturing
the GBA video output (and passing it to the television set), and in the other
direction, passing the Gamecube joypad input to the GBA inputs.

Unlocking and Detecting Gameboy Player Functions

Both unlocking and detection requires to display the 240x160 pixel Gameboy
Player logo (44 colors) for a number of frames... maybe at least 3-4 frames?
not sure if it checks the color of the logo... so maybe it can be hidden by
using dark gray on black background?

While displaying this logo, the joypad data will switch between values 03FFh (2
frames duration) and 030Fh (1 frame duration). The latter value (left, right,
up, down all pressed) indicates that it's a Gameboy Player.

Palette

Knowing Nintendo, they've probably not reproduced the blurred GBA colors (?),
so the games won't look as desired on the TV screen. Unless the game does
detect the Gameboy Player, and adjust the colors accordingly by software.

Rumble

The only known existing special function is the joypad rumble function,
controlled by sending data through the serial port (the normal GBA port, even
though it also has the connectors).

The Game Boy Player added a rumble feature to certain Game Boy Advance games
when played with a GameCube controller. Those games included:

 Drill Dozer (supports BOTH handheld-rumble and GBP-rumble?)
 Mario & Luigi: Superstar Saga
 Pokemon Pinball: Ruby & Sapphire
 Shikakui Atama wo Marukusuru Advance: Kokugo Sansu Rika Shakai
 Shikakui Atama wo Marukusuru Advance: Kanji Keisan
 Summon Night Craft Sword Monogatari: Hajimari no Ishi
 Super Mario Advance 4: Super Mario Bros. 3

Fredrik Olsson (aka Flubba) has implemented rumble in 3 applications now
RumblePong (FluBBA) (homebrew)

  Remudvance (FluBBA) (homebrew)
  Goomba (FluBBA) (8bit Gameboy Color Emulator for 32bit GBA) (homebrew)
  and, supposedly in "Tetanus on Drugs" (Tepples) (homebrew)

The GBP can also use some of the extra controllers for the GC like the Bongas

from Donkey Konga.

The logo requires at least 256 colors, it doesn't matter if you use a tiled

screen mode or a bitmapped one, the logo can be ripped from either

"Pokemon Pinball" or "Super Mario Advance 4".

Rumble

After detecting/unlocking the Gameboy Player, init RCNT and SIOCNT to 32bit
normal mode, external clock, SO=high, with IRQ enabled, and set the transfer
start bit. You should then receive the following sequence (about once per
frame), and your serial IRQ handler should send responses accordingly:

  Receive  Response
  0000494E 494EB6B1
  xxxx494E 494EB6B1
  B6B1494E 544EB6B1
  B6B1544E 544EABB1
  ABB1544E 4E45ABB1
  ABB14E45 4E45B1BA
  B1BA4E45 4F44B1BA
  B1BA4F44 4F44B0BB
  B0BB4F44 8000B0BB
  B0BB8002 10000010
  10000010 20000013
  20000013 40000004
  30000003 40000004
  30000003 40000004
  30000003 40000004
  30000003 400000yy
  30000003 40000004

The first part of the transfer just contains the string "NINTENDO" split into
16bit fragments, and bitwise inversions thereof (eg. 494Eh="NI", and B6B1h=NOT
494Eh). In the second part, <yy> should be 04h=RumbleOff, or
26h=RumbleOn.

Note

If it's having a similar range of functions as the 8bit Super Gameboy, then the
Gameboy Player might be also able to access analogue joypad input, and to
access other features of the Gamecube hardware, up to possibly executing code
on the Gamecube CPU...?
```
