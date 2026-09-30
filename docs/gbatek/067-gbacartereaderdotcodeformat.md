# GBA Cart e-Reader Dotcode Format

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderdotcodeformat

```
Resolution is 342.39 DPI (almost 10 blocks per inch).

Resolution is 134.8 dots/cm (almost 4 blocks per centimeter).

The width and height of each block, and the spacing to the bottom edge of the
card is ca. 1/10 inch, or ca. 4 millimeters.

   XXX            BLOCK 1             XXX            BLOCK 2             XXX
  XXXXX                              XXXXX                              XXXXX
  XXXXX   X X X X X X  X X X X X X   XXXXX   X X X X X X  X X X X X X   XXXXX
  XXXXX                              XXXXX                              XXXXX
   XXX   HHHHHHHHHHHHHHHHHHHH......   XXX   HHHHHHHHHHHHHHHHHHHH......   XXX
         ..........................         ..........................
         ...... 3 short lines .....         ..........................
    A..................................A..................................A..
    A....      26 long lines       ....A........ X = Sync Marks   ........A..
    A....  (each 34 data dots)     ....A........ H = Block Header ........A..
    A....(not all lines shown here)....A........ . = Data Bits    ........A..
    A..................................A........ A = Address Bits ........A..
         ...... 3 short lines .....         ..........................
         ...(each 26 data dots)....         ..........................
   XXX   ..........................   XXX   ..........................   XXX
  XXXXX                              XXXXX                              XXXXX
  XXXXX   X X X X X X  X X X X X X   XXXXX   X X X X X X  X X X X X X   XXXXX
  XXXXX                              XXXXX                              XXXXX
   XXX                                XXX                                XXX
             <ca. 35 blank lines>
  ___Snip____________________________________________________________________

Address Columns

Each Column consists of 26 dots. From top to bottom: 1 black dot, 8 blank dots,
16 address dots (MSB topmost), and 1 blank dot. The 16bit address values can be
calculated as:

  addr[0] = 03FFh
  for i = 1 to 53
    addr[i] = addr[i-1] xor ((i and (-i)) * 769h)
    if (i and 07h)=0 then addr[i] = addr[i] xor (769h)
    if (i and 0Fh)=0 then addr[i] = addr[i] xor (769h*2)
    if (i and 1Fh)=0 then addr[i] = addr[i] xor (769h*4) xor (769h)
  next i

Short strips use addr[1..19], long strips use addr[25..53], left to right.

Block Header

The 18h-byte Block Header is taken from the 1st two bytes (20 dots) of the 1st
0Ch blocks (and is then repeated in the 1st two bytes of further blocks).

  00h      Unknown              (00h)
  01h      Dotcode type         (02h=Short, 03h=Long)
  02h      Unknown              (00h)
  03h      Address of 1st Block (01h=Short, 19h=Long)
  04h      Total Fragment Size  (40h) ;64 bytes per fragment, of which,
                                      ;48 bytes are actual data, the remaining
  05h      Error-Info Size      (10h) ;16 bytes are error-info
  06h      Unknown              (00h)
  07h      Interleave Value     (1Ch=Short, 2Ch=Long)
  08h..17h 16 bytes Reed-solomon error correction info for Block Header

Data 4-Bit to 5-bit Conversion

In the Block Header (HHHHH), and Data Region (.....), each 4bit are expanded to
5bit, so one byte occupies 10 dots, and each block (1040 data dots) contains
104 bytes.

  4bit  00h 01h 02h 03h 04h 05h 06h 07h 08h 09h 0Ah 0Bh 0Ch 0Dh 0Eh 0Fh
  5bit  00h 01h 02h 12h 04h 05h 06h 16h 08h 09h 0Ah 14h 0Ch 0Dh 11h 10h

That formatting ensures that there are no more than two continous black dots
(in horizontal direction), neither inside of a 5bit value, nor between two 5bit
values, however, the address bars are violating that rule, and up to 5
continous black dots can appear at the (..A..) block boundaries.

Data Order

Data starts with the upper bit of the 5bit value for the upper 4bit of the
first byte, which is located at the leftmost dot of the upper line of the
leftmost block, it does then extend towards rightmost dot of that block, and
does then continue in the next line, until reaching the bottom of the block,
and does then continue in the next block. The 1st two bytes of each block
contain a portion of the Block Header, the remaining 102 bytes in each block
contain data.

Data Size

A long strip consists of 28 blocks (28*104 = 2912 bytes), a short strip of 18
blocks (18*104 = 1872 bytes). Of which, less than 75% can be actually used for
program code, the remaining data contains error correction info, and various
headers. See Data Format for more info.

Interleaved Fragments

The Interleave Value (I) specifies the number of fragments, and does also
specify the step to the next byte inside of a fragment; except that, at the
block boundaries (every 104 bytes), the step is 2 bigger (for skipping the next
two Block Header bytes).

  RAW Offset  Content
  000h..001h  1st 2 bytes of RAW Header
  002h        1st byte of 1st fragment
  003h        1st byte of 2nd fragment
  ...         ...
  002h+I-1    1st byte of last fragment
  002h+I      2nd byte of 1st fragment
  003h+I      2nd byte of 2nd fragment
  ...         ...
  002h+I*2-1  2nd byte of last fragment
  ...         ...

Each fragment consists of 48 actual data bytes, followed by 16 error correction
bytes, followed by 0..2 unused bytes (since I*40h doesn't exactly match
num_blocks*102).
```
