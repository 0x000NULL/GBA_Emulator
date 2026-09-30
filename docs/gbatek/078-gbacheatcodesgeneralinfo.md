# GBA Cheat Codes - General Info

Source: https://problemkaputt.de/gbatek.htm#gbacheatcodesgeneralinfo

```
Cheat devices are external adapters, connected between the GBA and the game
cartridge. The devices include a BIOS ROM which is, among others, used to
prompt the user to enter cheat codes.

These codes are used to patch specified memory locations for a certain GBA
game, allowing the user to gain goodies such like Infinite sex, 255 Cigarettes,
etc.

ROM and RAM Patches

For ROM Patches, the device watches the address bus, if it matches a specified
address then it outputs a patched value to the data bus, that mechanism is
implemented by hardware, aside from the Hook Enable Code some devices also
allow a limited number of cheats to use ROM patches.

Most cheat codes are RAM patches, each time when the hook procedure is executed
it will process all codes and overwrite the specified addresses in RAM (or VRAM
or I/O area) by the desired values.

Enable Codes (Must Be On)

Enable codes usually consist of the Game ID, Hook Address, and eventually a
third code used to encrypt all following codes. The Game ID is used to confirm
that the correct cartridge is inserted, just a verification, though the device
may insist on the ID code.

The Hook Address specifies an address in cartridge ROM, and should point to an
opcode which is executed several times per second (eg. once per frame, many
codes place the hook in the joypad handler). At the hook address, the device
redirects to its own BIOS, processes the RAM patches, and does then return
control to the game cartridge.

Note: The hook address should not point to opcodes with relative addressing
(eg. B, BL, LDR Rd,=Imm, ADD Rd,=Imm opcodes - which are all relative to PC
program counter register).

Alignment

Addresses for 16bit or 32bit values should be properly aligned.
```
