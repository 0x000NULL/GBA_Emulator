# GBA Comparison of CGB and GBA Sound

Source: https://problemkaputt.de/gbatek.htm#gbacomparisonofcgbandgbasound

```
The GBA sound controller is mostly the same than that of older monochrome
gameboy and CGB. The following changes have been done:

New Sound Channels

Two new sound channels have been added that may be used to replay 8bit digital
sound. Sample rate and sample data must be supplied by using a Timer and a DMA
channel.

New Control Registers

The SOUNDCNT_H register controls the new DMA channels - as well as mixing with
the four old channels. The SOUNDBIAS register controls the final sound output.

Sound Channel 3 Changes

The length of the Wave RAM is doubled by dividing it into two banks of 32
digits each, either one or both banks may be replayed (one after each other),
for details check NR30 Bit 5-6. Optionally, the sound may be output at 75%
volume, for details check NR32 Bit 7.

Changed Control Registers

NR50 is not supporting Vin signals (that's been an external sound input from
cartridge).

Changed I/O Addresses

The GBAs sound register are located at 04000060-040000AE instead of at
FF10-FF3F as in CGB and monochrome gameboy. However, note that there have been
new blank spaces inserted between some of the separate registers - therefore it
is NOT possible to port CGB software to GBA just by changing the sound base
address.

Accessing I/O Registers

In some cases two of the old 8bit registers are packed into a 16bit register
and may be accessed as such.
```
