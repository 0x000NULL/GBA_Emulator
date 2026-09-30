# BIOS Decompression Functions

Source: https://problemkaputt.de/gbatek.htm#biosdecompressionfunctions

```
BitUnPack

Diff8bitUnFilter

HuffUnComp

LZ77UnComp

RLUnComp

Decompression Read/Write Variants

  ReadNormal:      Fast (src must be memory mapped)
  ReadByCallback:  Slow (src can be non-memory, eg. serial Firmware SPI bus)
  Write8bitUnits:  Fast (dest must support 8bit writes, eg. not VRAM)
  Write16bitUnits: Slow (dest must be halfword-aligned) (for VRAM)

BitUnPack - SWI 10h (GBA/NDS7/NDS9/DSi7/DSi9)

Used to increase the color depth of bitmaps or tile data. For example, to
convert a 1bit monochrome font into 4bit or 8bit GBA tiles. The Unpack Info is
specified separately, allowing to convert the same source data into different
formats.

  r0  Source Address      (no alignment required)
  r1  Destination Address (must be 32bit-word aligned)
  r2  Pointer to UnPack information:
       16bit  Length of Source Data in bytes     (0-FFFFh)
       8bit   Width of Source Units in bits      (only 1,2,4,8 supported)
       8bit   Width of Destination Units in bits (only 1,2,4,8,16,32 supported)
       32bit  Data Offset (Bit 0-30), and Zero Data Flag (Bit 31)
      The Data Offset is always added to all non-zero source units.
      If the Zero Data Flag was set, it is also added to zero units.

Data is written in 32bit units, Destination can be Wram or Vram. The size of
unpacked data must be a multiple of 4 bytes. The width of source units (plus
the offset) should not exceed the destination width.

Return: No return value, Data written to destination address.

Diff8bitUnFilterWrite8bit (Wram) - SWI 16h (GBA/NDS9/DSi9)

Diff8bitUnFilterWrite16bit (Vram) - SWI 17h (GBA)

Diff16bitUnFilter - SWI 18h (GBA/NDS9/DSi9)

These aren't actually real decompression functions, destination data will have
exactly the same size as source data. However, assume a bitmap or wave form to
contain a stream of increasing numbers such like 10..19, the
filtered/unfiltered data would be:

  unfiltered:   10  11  12  13  14  15  16  17  18  19
  filtered:     10  +1  +1  +1  +1  +1  +1  +1  +1  +1

In this case using filtered data (combined with actual compression algorithms)
will obviously produce better compression results.

Data units may be either 8bit or 16bit used with Diff8bit or Diff16bit
functions respectively.

  r0  Source address (must be aligned by 4) pointing to data as follows:
       Data Header (32bit)
         Bit 0-3   Data size (must be 1 for Diff8bit, 2 for Diff16bit)
         Bit 4-7   Type (must be 8 for DiffFiltered)
         Bit 8-31  24bit size after decompression
       Data Units (each 8bit or 16bit depending on used SWI function)
         Data0          ;original data
         Data1-Data0    ;difference data
         Data2-Data1    ;...
         Data3-Data2
         ...
  r1  Destination address

Return: No return value, Data written to destination address.

HuffUnCompReadNormal - SWI 13h (GBA)

HuffUnCompReadByCallback - SWI 13h (NDS/DSi)

The decoder starts in root node, the separate bits in the bitstream specify if
the next node is node0 or node1, if that node is a data node, then the data is
stored in memory, and the decoder is reset to the root node. The most often
used data should be as close to the root node as possible. For example, the
4-byte string "Huff" could be compressed to 6 bits: 10-11-0-0, with root.0
pointing directly to data "f", and root.1 pointing to a child node, whose nodes
point to data "H" and data "u".

Data is written in units of 32bits, if the size of the compressed data is not a
multiple of 4, please adjust it as much as possible by padding with 0.

Align the source address to a 4Byte boundary.

  r0  Source Address, aligned by 4, pointing to:
       Data Header (32bit)
         Bit0-3   Data size in bit units (normally 4 or 8)
         Bit4-7   Compressed type (must be 2 for Huffman)
         Bit8-31  24bit size of decompressed data in bytes
       Tree Size (8bit)
         Bit0-7   Size of Tree Table/2-1 (ie. Offset to Compressed Bitstream)
       Tree Table (list of 8bit nodes, starting with the root node)
        Root Node and Non-Data-Child Nodes are:
         Bit0-5   Offset to next child node,
                  Next child node0 is at (CurrentAddr AND NOT 1)+Offset*2+2
                  Next child node1 is at (CurrentAddr AND NOT 1)+Offset*2+2+1
         Bit6     Node1 End Flag (1=Next child node is data)
         Bit7     Node0 End Flag (1=Next child node is data)
        Data nodes are (when End Flag was set in parent node):
         Bit0-7   Data (upper bits should be zero if Data Size is less than 8)
       Compressed Bitstream (stored in units of 32bits)
         Bit0-31  Node Bits (Bit31=First Bit)  (0=Node0, 1=Node1)
  r1  Destination Address
  r2  Callback temp buffer      ;\for NDS/DSi "ReadByCallback" variants only
  r3  Callback structure        ;/(see Callback notes below)

Return: No return value, Data written to destination address.

LZ77UnCompReadNormalWrite8bit (Wram) - SWI 11h (GBA/NDS7/NDS9/DSi7/DSi9)

LZ77UnCompReadNormalWrite16bit (Vram) - SWI 12h (GBA)

LZ77UnCompReadByCallbackWrite8bit - SWI 01h (DSi7/DSi9)

LZ77UnCompReadByCallbackWrite16bit - SWI 12h (NDS), SWI 02h or 19h (DSi)

Expands LZ77-compressed data. The Wram function is faster, and writes in units
of 8bits. For the Vram function the destination must be halfword aligned, data
is written in units of 16bits.

LZ Decompression Functions

CAUTION: Writing 16bit units to [dest-1] instead of 8bit units to [dest] means
that reading from [dest-1] won't work, ie. the "Vram" function works only with
disp=001h..FFFh, but not with disp=000h.

If the size of the compressed data is not a multiple of 4, please adjust it as
much as possible by padding with 0. Align the source address to a 4-Byte
boundary.

  r0  Source address, pointing to data as such:
       Data header (32bit)
         Bit 0-3   Reserved (0)
         Bit 4-7   Compressed type (must be 1 for LZ77)
         Bit 8-31  Size of decompressed data
       Repeat below. Each Flag Byte followed by eight Blocks.
       Flag data (8bit)
         Bit 0-7   Type Flags for next 8 Blocks, MSB first
       Block Type 0 - Uncompressed - Copy 1 Byte from Source to Dest
         Bit 0-7   One data byte to be copied to dest
       Block Type 1 - Compressed - Copy N+3 Bytes from Dest-Disp-1 to Dest
         Bit 0-3   Disp MSBs
         Bit 4-7   Number of bytes to copy (minus 3)
         Bit 8-15  Disp LSBs
  r1  Destination address
  r2  Callback parameter        ;\for NDS/DSi "ReadByCallback" variants only
  r3  Callback structure        ;/(see Callback notes below)

Return: No return value.

RLUnCompReadNormalWrite8bit (Wram) - SWI 14h (GBA/NDS7/NDS9/DSi7/DSi9)

RLUnCompReadNormalWrite16bit (Vram) - SWI 15h (GBA)

RLUnCompReadByCallbackWrite16bit - SWI 15h (NDS7/NDS9/DSi7/DSi9)

Expands run-length compressed data. The Wram function is faster, and writes in
units of 8bits. For the Vram function the destination must be halfword aligned,
data is written in units of 16bits.

If the size of the compressed data is not a multiple of 4, please adjust it as
much as possible by padding with 0. Align the source address to a 4Byte
boundary.

  r0  Source Address, pointing to data as such:
       Data header (32bit)
         Bit 0-3   Reserved
         Bit 4-7   Compressed type (must be 3 for run-length)
         Bit 8-31  Size of decompressed data
       Repeat below. Each Flag Byte followed by one or more Data Bytes.
       Flag data (8bit)
         Bit 0-6   Expanded Data Length (uncompressed N-1, compressed N-3)
         Bit 7     Flag (0=uncompressed, 1=compressed)
       Data Byte(s) - N uncompressed bytes, or 1 byte repeated N times
  r1  Destination Address
  r2  Callback parameter        ;\for NDS/DSi "ReadByCallback" variants only
  r3  Callback structure        ;/(see Callback notes below)

Return: No return value, Data written to destination address.

NDS/DSi Decompression Callbacks

On NDS and DSi, the "ReadByCallback" variants are reading source data from
callback functions (rather than directly from memory). The callback functions
may read normal data from memory, or from other devices, such like directly
from the gamepak bus, without storing the source data in memory. The downside
is that the callback mechanism makes the function very slow, furthermore,
NDS7/NDS9 SWI 12h, 13h, 15h are using THUMB code, and variables on stack,
alltogether that makes the whole shit very-very-very slow.

  r2 = user defined callback parameter (passed on to Open function)
        (or, for Huffman: pointer to temp buffer, max 200h bytes needed)
  r3 = pointer to callback structure

Callback structure (five 32bit pointers to callback functions)

  Open_and_get_32bit (eg. LDR r0,[r0], get header)
  Close              (optional, 0=none)
  Get_8bit           (eg. LDRB r0,[r0])
  Get_16bit          (not used)
  Get_32bit          (used by Huffman only)

All functions may use ARM or THUMB code (indicated by address bit0). The
current source address (r0) is passed to all callback functions. Additionally,
the initial destination address (r1), and a user defined parameter (r2) are
passed to the Open function. For Huffman r2 must point to a temp buffer (max
200h bytes needed, internally used by the SWI function to make a copy of the
huffman tree; needed for random-access to the tree, which wouldn't work with
the sequentially reading callbacks).

All functions have return values in r0. The Open function normally returns the
first word (containing positive length and type), alternatively it may return a
negative error code to abort/reject decompression. The Close function, if it is
defined, should return zero (or any positive value), or a negative errorcode.
The other functions return raw data, without errorcodes. The SWI returns the
length of decompressed data, or the signed errorcode from the Open/Close
functions.
```
