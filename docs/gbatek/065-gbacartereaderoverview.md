# GBA Cart e-Reader Overview

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderoverview

```
The e-Reader is a large GBA cartridge (about as big as the GBA console), with
built-in dotcode scanning hardware. Dotcodes are tiny strips of black and white
pixels printed on the edges of cardboard cards. The cards have to be pulled
through a slot on the e-Reader, which is giving it a feeling like using a
magnet card reader. The binary data on the dotcodes contains small games,
either in native GBA code (ARM/THUMB), or in software emulated 8bit Z80 or
NES/Famicom (6502) code.

The e-Reader Hardware

The hardware consists of regular 8MByte ROM and 128KByte FLASH chips, two link
ports, a custom PGA chip, the camera module (with two red LEDs, used as light
source), and some analogue components for generating the LED voltages, etc. The
camera supports 402x302 pixels with 7bit monochrome color depth, but the PGA
clips it to max 320 pixels per scanline with 1bit color depth.

Link Port Plug/Socket

The e-Reader's two link ports are simply interconnected with each other;
without connection to the rest of the e-Reader hardware. These ports are used
only on the original GBA (where the large e-Reader cartridge would be covering
the GBA's link socket). When trying to insert the e-Reader into an original NDS
(or GBA-Micro), then the e-Reader's link plug will hit against the case of the
NDS, so it works only with some minor modification to the hardware. There's no
such problem with GBA-SP and NDS-Lite.

Region/Version

There are 3 different e-Reader's: Japanese/Original, Japanese/Plus, and
Non-Japanese. The Original version has only 64K FLASH, no Link Port, and
reportedly supports only Z80 code, but no NES/GBA code. The Plus and
Non-Japanese versions should be almost identical, except that they reject cards
from the wrong region, and that the title strings aren't ASCII in Japan, the
Plus version should be backwards compatible to the Original one.

The Problem

Nintendo's current programmers are definetly unable to squeeze a Pac-Man style
game into less than 4MBytes. Their solution has been: MORE memory. That is,
they've put a whopping 8MByte BIOS ROM into the e-Reader, which contains the
User Interface, and software emulation for running some of their 20 years old
8bit NES and Game&Watch titles, which do fit on a few dotcode strips.
```
