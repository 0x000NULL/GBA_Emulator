# GBA Cartridge Header

Source: https://problemkaputt.de/gbatek.htm#gbacartridgeheader

```
The first 192 bytes at 8000000h-80000BFh in ROM are used as cartridge header.
The same header is also used for Multiboot images at 2000000h-20000BFh (plus
some additional multiboot entries at 20000C0h and up).

Header Overview

  Address Bytes Expl.
  000h    4     ROM Entry Point  (32bit ARM branch opcode, eg. "B rom_start")
  004h    156   Nintendo Logo    (compressed bitmap, required!)
  0A0h    12    Game Title       (uppercase ascii, max 12 characters)
  0ACh    4     Game Code        (uppercase ascii, 4 characters)
  0B0h    2     Maker Code       (uppercase ascii, 2 characters)
  0B2h    1     Fixed value      (must be 96h, required!)
  0B3h    1     Main unit code   (00h for current GBA models)
  0B4h    1     Device type      (usually 00h) (bit7=DACS/debug related)
  0B5h    7     Reserved Area    (should be zero filled)
  0BCh    1     Software version (usually 00h)
  0BDh    1     Complement check (header checksum, required!)
  0BEh    2     Reserved Area    (should be zero filled)
  --- Additional Multiboot Header Entries ---
  0C0h    4     RAM Entry Point  (32bit ARM branch opcode, eg. "B ram_start")
  0C4h    1     Boot mode        (init as 00h - BIOS overwrites this value!)
  0C5h    1     Slave ID Number  (init as 00h - BIOS overwrites this value!)
  0C6h    26    Not used         (seems to be unused)
  0E0h    4     JOYBUS Entry Pt. (32bit ARM branch opcode, eg. "B joy_start")

Note: With all entry points, the CPU is initially set into system mode.

000h - Entry Point, 4 Bytes

Space for a single 32bit ARM opcode that redirects to the actual startaddress
of the cartridge, this should be usually a "B <start>" instruction.

Note: This entry is ignored by Multiboot slave GBAs (in fact, the entry is then
overwritten and redirected to a separate Multiboot Entry Point, as described
below).

004h..09Fh - Nintendo Logo, 156 Bytes

Contains the Nintendo logo which is displayed during the boot procedure.
Cartridge won't work if this data is missing or modified.

In detail: This area contains Huffman compression data (but excluding the
compression header which is hardcoded in the BIOS, so that it'd be probably not
possible to hack the GBA by producing de-compression buffer overflows).

A copy of the compression data is stored in the BIOS, the GBA will compare this
data and lock-up itself if the BIOS data isn't exactly the same as in the
cartridge (or multiboot header). The only exception are the two entries below
which are allowed to have variable settings in some bits.

09Ch Bit 2,7 - Debugging Enable

This is part of the above Nintendo Logo area, and must be commonly set to 21h,
however, Bit 2 and Bit 7 may be set to other values.

When both bits are set (ie. A5h), the FIQ/Undefined Instruction handler in the
BIOS becomes unlocked, the handler then forwards these exceptions to the user
handler in cartridge ROM (entry point defined in 80000B4h, see below).

Other bit combinations currently do not seem to have special functions.

09Eh Bit 0,1 - Cartridge Key Number MSBs

This is part of the above Nintendo Logo area, and must be commonly set to F8h,
however, Bit 0-1 may be set to other values.

During startup, the BIOS performs some dummy-reads from a stream of pre-defined
addresses, even though these reads seem to be meaningless, they might be
intended to unlock a read-protection inside of commercial cartridge. There are
16 pre-defined address streams - selected by a 4bit key number - of which the
upper two bits are gained from 800009Eh Bit 0-1, and the lower two bits from a
checksum across header bytes 09Dh..0B7h (bytewise XORed, divided by 40h).

0A0h - Game Title, Uppercase Ascii, max 12 characters

Space for the game title, padded with 00h (if less than 12 chars).

0ACh - Game Code, Uppercase Ascii, 4 characters

This is the same code as the AGB-UTTD code which is printed on the package and
sticker on (commercial) cartridges (excluding the leading "AGB-" part).

  U  Unique Code          (usually "A" or "B" or special meaning)
  TT Short Title          (eg. "PM" for Pac Man)
  D  Destination/Language (usually "J" or "E" or "P" or specific language)

The first character (U) is usually "A" or "B", in detail:

  A  Normal game; Older titles (mainly 2001..2003)
  B  Normal game; Newer titles (2003..)
  C  Normal game; Not used yet, but might be used for even newer titles
  F  Famicom/Classic NES Series (software emulated NES games)
  K  Yoshi and Koro Koro Puzzle (acceleration sensor)
  P  e-Reader (dot-code scanner) (or NDS PassMe image when gamecode="PASS")
  R  Warioware Twisted (cartridge with rumble and z-axis gyro sensor)
  U  Boktai 1 and 2 (cartridge with RTC and solar sensor)
  V  Drill Dozer (cartridge with rumble)

The second/third characters (TT) are:

  Usually an abbreviation of the game title (eg. "PM" for "Pac Man") (unless
  that gamecode was already used for another game, then TT is just random)

The fourth character (D) indicates Destination/Language:

  J  Japan             P  Europe/Elsewhere   F  French          S  Spanish
  E  USA/English       D  German             I  Italian

0B0h - Maker code, Uppercase Ascii, 2 characters

Identifies the (commercial) developer. For example, "01"=Nintendo.

0B2h - Fixed value, 1 Byte

Must be 96h.

0B3h - Main unit code, 1 Byte

Identifies the required hardware. Should be 00h for current GBA models.

0B4h - Device type, 1 Byte

Normally, this entry should be zero. With Nintendo's hardware debugger Bit 7
identifies the debugging handlers entry point and size of DACS (Debugging And
Communication System) memory: Bit7=0: 9FFC000h/8MBIT DACS, Bit7=1:
9FE2000h/1MBIT DACS. The debugging handler can be enabled in 800009Ch (see
above), normal cartridges do not have any memory (nor any mirrors) at these
addresses though.

0B5h - Reserved Area, 7 Bytes

Reserved, zero filled.

0BCh - Software version number

Version number of the game. Usually zero.

0BDh - Complement check, 1 Byte

Header checksum, cartridge won't work if incorrect. Calculate as such:

chk=0:for i=0A0h to 0BCh:chk=chk-[i]:next:chk=(chk-19h) and 0FFh

0BEh - Reserved Area, 2 Bytes

Reserved, zero filled.

Below required for Multiboot/slave programs only. For Multiboot, the above 192
bytes are required to be transferred as header-block (loaded to
2000000h-20000BFh), and some additional header-information must be located at
the beginning of the actual program/data-block (loaded to 20000C0h and up).
This extended header consists of Multiboot Entry point(s) which must be set up
correctly, and of two reserved bytes which are overwritten by the boot
procedure:

0C0h - Normal/Multiplay mode Entry Point

This entry is used only if the GBA has been booted by using Normal or Multiplay
transfer mode (but not by Joybus mode).

Typically deposit a ARM-32bit "B <start>" branch opcode at this location,
which is pointing to your actual initialization procedure.

0C4h (BYTE) - Boot mode

The slave GBA download procedure overwrites this byte by a value which is
indicating the used multiboot transfer mode.

  Value  Expl.
  01h    Joybus mode
  02h    Normal mode
  03h    Multiplay mode

Typically set this byte to zero by inserting DCB 00h in your source.

Be sure that your uploaded program does not contain important program code or
data at this location, or at the ID-byte location below.

0C5h (BYTE) - Slave ID Number

If the GBA has been booted in Normal or Multiplay mode, this byte becomes
overwritten by the slave ID number of the local GBA (that'd be always 01h for
normal mode).

  Value  Expl.
  01h    Slave #1
  02h    Slave #2
  03h    Slave #3

Typically set this byte to zero by inserting DCB 00h in your source.

When booted in Joybus mode, the value is NOT changed and remains the same as
uploaded from the master GBA.

0C6h..0DFh - Not used

Appears to be unused.

0E0h - Joybus mode Entry Point

If the GBA has been booted by using Joybus transfer mode, then the entry point
is located at this address rather than at 20000C0h. Either put your
initialization procedure directly at this address, or redirect to the actual
boot procedure by depositing a "B <start>" opcode here (either one using
32bit ARM code). Or, if you are not intending to support joybus mode (which is
probably rarely used), ignore this entry.
```
