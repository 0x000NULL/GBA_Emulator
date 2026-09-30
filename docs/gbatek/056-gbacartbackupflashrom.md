# GBA Cart Backup Flash ROM

Source: https://problemkaputt.de/gbatek.htm#gbacartbackupflashrom

```
64 KBytes - 512Kbits Flash ROM - Lifetime: 10,000 writes per sector

128 KBytes - 1Mbit Flash ROM - Lifetime: ??? writes per sector

Chip Identification (all device types)

  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=90h  (enter ID mode)
  dev=[E000001h], man=[E000000h]                  (get device & manufacturer)
  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=F0h  (terminate ID mode)

Used to detect the type (and presence) of FLASH chips. See Device Types below.

Reading Data Bytes (all device types)

  dat=[E00xxxxh]                                  (read byte from address xxxx)

Erase Entire Chip (all device types)

  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=80h  (erase command)
  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=10h  (erase entire chip)
  wait until [E000000h]=FFh (or timeout)

Erases all memory in chip, erased memory is FFh-filled.

Erase 4Kbyte Sector (all device types, except Atmel)

  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=80h  (erase command)
  [E005555h]=AAh, [E002AAAh]=55h, [E00n000h]=30h  (erase sector n)
  wait until [E00n000h]=FFh (or timeout)

Erases memory at E00n000h..E00nFFFh, erased memory is FFh-filled.

Erase-and-Write 128 Bytes Sector (only Atmel devices)

  old=IME, IME=0                                  (disable interrupts)
  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=A0h  (erase/write sector command)
  [E00xxxxh+00h..7Fh]=dat[00h..7Fh]               (write 128 bytes)
  IME=old                                         (restore old IME state)
  wait until [E00xxxxh+7Fh]=dat[7Fh] (or timeout)

Interrupts (and DMAs) should be disabled during command/write phase. Target
address must be a multiple of 80h.

Write Single Data Byte (all device types, except Atmel)

  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=A0h  (write byte command)
  [E00xxxxh]=dat                                  (write byte to address xxxx)
  wait until [E00xxxxh]=dat (or timeout)

The target memory location must have been previously erased.

Terminate Command after Timeout (only Macronix devices, ID=1CC2h)

  [E005555h]=F0h                            (force end of write/erase command)

Use if timeout occurred during "wait until" periods, for Macronix devices only.

Bank Switching (devices bigger than 64K only)

  [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=B0h  (select bank command)
  [E000000h]=bnk                                  (write bank number 0..1)

Specifies 64K bank number for read/write/erase operations.

Required because gamepak flash/sram addressbus is limited to 16bit width.

Device Types

Nintendo puts different FLASH chips in commercial game cartridges. Developers
should thus detect & support all chip types. For Atmel chips it'd be
recommended to simulate 4K sectors by software, though reportedly Nintendo
doesn't use Atmel chips in newer games anymore. Also mind that different
timings should not disturb compatibility and performance.

  ID     Name       Size  Sectors  AverageTimings  Timeouts/ms   Waits
  D4BFh  SST        64K   16x4K    20us?,?,?       10,  40, 200  3,2
  1CC2h  Macronix   64K   16x4K    ?,?,?           10,2000,2000  8,3
  1B32h  Panasonic  64K   16x4K    ?,?,?           10, 500, 500  4,2
  3D1Fh  Atmel      64K   512x128  ?,?,?           ...40..,  40  8,8
  1362h  Sanyo      128K  ?        ?,?,?           ?    ?    ?    ?
  09C2h  Macronix   128K  ?        ?,?,?           ?    ?    ?    ?

Identification Codes MSB=Device Type, LSB=Manufacturer.

Size in bytes, and numbers of sectors * sector size in bytes.

Average medium Write, Erase Sector, Erase Chips timings are unknown?

Timeouts in milliseconds for Write, Erase Sector, Erase Chips.

Waitstates for Writes, and Reads in clock cycles.

Accessing FLASH Memory

FLASH memory is located in the "SRAM" area at E000000h..E00FFFFh, which is
restricted to 16bit address and 8bit data buswidths. Respectively, the memory
can be accessed <only> by 8bit read/write LDRB/STRB opcodes.

Also, reading anything (data or status/busy information) can be done
<only> by opcodes executed in WRAM (not from opcodes in ROM) (there's no
such restriction for writing).

FLASH Waitstates

Use 8 clk waitstates for initial detection (WAITCNT Bits 0,1 both set). After
detection of certain device types smaller wait values may be used for
write/erase, and even smaller wait values for raw reading, see Device Types
table.

In practice, games seem to use smaller values only for write/erase (even though
those operations are slow anyways), whilst raw reads are always done at 8 clk
waits (even though reads could actually benefit slightly from smaller wait
values).

Verify Write/Erase and Retry

Even though device signalizes the completion of write/erase operations, it'd be
recommended to read/confirm the content of the changed memory area by software.
In practice, Nintendo's "erase-write-verify-retry" function typically repeats
the operation up to three times in case of errors.

Also, for SST devices only, the "erase-write" and "erase-write-verify-retry"
functions repeat the erase command up to 80 times, additionally followed by one
further erase command if no retries were needed, otherwise followed by six
further erase commands.

Note

FLASH (64Kbytes) is used by the game Sonic Advance, and possibly others.
```
