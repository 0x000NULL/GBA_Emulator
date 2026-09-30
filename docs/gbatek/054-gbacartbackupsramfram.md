# GBA Cart Backup SRAM/FRAM

Source: https://problemkaputt.de/gbatek.htm#gbacartbackupsramfram

```
SRAM - 32 KBytes (256Kbit) Lifetime: Depends on back-up battery

FRAM - 32 KBytes (256Kbit) Lifetime: 10,000,000,000 read/write per bit

Hyundai GM76V256CLLFW10 SRAM (Static RAM) (eg. F-Zero)

Fujitsu MB85R256 FRAM (Ferroelectric RAM) (eg. Warioware Twisted)

Addressing and Waitstates

SRAM/FRAM is mapped to E000000h-E007FFFh, it should be accessed with 8
waitstates (write a value of 3 into Bit0-1 of WAITCNT).

Databus Width

The SRAM/FRAM databus is restricted to 8 bits, it should be accessed by LDRB,
LDRSB, and STRB opcodes only.

Reading and Writing

Reading from SRAM/FRAM should be performed by code executed in WRAM only (but
not by code executed in ROM). There is no such restriction for writing.

Preventing Data Loss

The GBA SRAM/FRAM carts do not include a write-protect function (unlike older
8bit gameboy carts). This seems to be a problem and may cause data loss when a
cartridge is removed or inserted while the GBA is still turned on. As far as I
understand, this is not so much a hardware problem, but rather a software
problem, ie. theoretically you could remove/insert the cartridge as many times
as you want, but you should take care that your program does not crash (and
write blindly into memory).

Recommended Workaround

Enable the Gamepak Interrupt (it'll most likely get triggered when removing the
cartridge), and hang-up the GBA in an endless loop when your interrupt handler
senses a Gamepak IRQ. For obvious reason, your interrupt handler should be
located in WRAM, ie. not in the (removed) ROM cartridge. The handler should
process Gamepak IRQs at highest priority. Periods during which interrupts are
disabled should be kept as short as possible, if necessary allow nested
interrupts.

When to use the above Workaround

A program that relies wholly on code and data in WRAM, and that does not crash
even when ROM is removed, may keep operating without having to use the above
mechanism.

Do NOT use the workaround for programs that run without a cartridge inserted
(ie. single gamepak/multiboot slaves), or for programs that use Gamepak IRQ/DMA
for other purposes.

All other programs should use it. It'd be eventually a good idea to include it
even in programs that do not use SRAM/FRAM themselves (eg. otherwise removing a
SRAM/FRAM-less cartridge may lock up the GBA, and may cause it to destroy
backup data when inserting a SRAM/FRAM cartridge).

SRAM vs FRAM

FRAM (Ferroelectric RAM) is a newer technology, used in newer GBA carts, unlike
SRAM (Static RAM), it doesn't require a battery to hold the data. At software
side, it is accessed exactly like SRAM, ie. unlike EEPROM/FLASH, it doesn't
require any Write/Erase commands/delays.

Note

In SRAM/FRAM cartridges, the /REQ pin (Pin 31 of Gamepak bus) should be a
little bit shorter as than the other pins; when removing the cartridge, this
causes the gamepak IRQ signal to get triggered before the other pins are
disconnected.
```
