# GBA Cart e-Reader VPK Decompression

Source: https://problemkaputt.de/gbatek.htm#gbacartereadervpkdecompression

```
vpk_decompress(src,dest)

  collected32bit=80000000h  ;initially empty (endflag in bit31)
  for i=0 to 3, id[i]=read_bits(8), next i, if id[0..3]<>'vpk0' then error
  dest_end=dest+read_bits(32)     ;size of decompressed data (of all strips)
  method=read_bits(8), if method>1 then error
  tree_index=0, load_huffman_tree, disproot=tree_index
  tree_index=tree_index+1, load_huffman_tree, lenroot=tree_index
  ;above stuff is contained only in the first strip. below loop starts at
  ;current location in first strip, and does then continue in further strips.
 decompress_loop:
  if read_bits(1)=0 then                   ;copy one uncompressed data byte,
    [dest]=read_bits(8), dest=dest+1       ;does work without huffman trees
  else
    if disproot=-1 or lenroot=-1 then error  ;compression does require trees
    disp=read_tree(disproot)
    if method=1   ;disp*4 is good for 32bit ARM opcodes
      if disp>2 then disp=disp*4-8 else disp=disp+4*read_tree(disproot)-7
    len=read_tree(lenroot)
    if len=0 or disp<=0 or dest+len-1>dest_end then error ;whoops
    for j=1 to len, [dest]=[dest-disp], dest=dest+1, next j
  if dest<dest_end then decompress_loop
  ret

read_bits(num)

  mov  data=0
  for i=1 to num
    shl collected32bit,1   ;move next bit to carry, or set zeroflag if empty
    if zeroflag
      collected32bit=[src+0]*1000000h+[src+1]*10000h+[src+2]*100h+[src+3]
      src=src+4            ;read data in 32bit units, in reversed byte-order
      carryflag=1          ;endbit
      rcl collected32bit,1 ;move bit31 to carry (and endbit to bit0)
    rcl data,1             ;move carry to data
  next i
  ret(data)

read_tree(root_index)

  i=root_index
  while node[i].right<>-1  ;loop until reaching data node
    if read_bits(1)=1 then i=node[i].right else i=node[i].left
  i=node[i].left           ;get number of bits
  i=read_bits(i)           ;read that number of bits
  ret(i)                   ;return that value

load_huffman_tree

  stacktop=sp
  if read_bits(1)=1 then tree_index=-1, ret  ;exit (empty)
  node[tree_index].right=-1                  ;indicate data node
  node[tree_index].left=read_bits(8)         ;store data value
  if read_bits(1)=1 then ret                 ;exit (only 1 data node at root)
  push tree_index                     ;save previous (child) node
  tree_index=tree_index+1
  jmp data_injump
 load_loop:
  push tree_index                     ;save previous (child) node
  tree_index=tree_index+1
  if read_bits(1)=1 then parent_node
 data_injump:
  node[tree_index].right=-1           ;indicate data node
  node[tree_index].left=read_bits(8)  ;store data value
  jmp load_loop
 parent_node:
  pop node[tree_index].right          ;store 1st child
  pop node[tree_index].left           ;store 2nd child
  if sp<>stacktop then jmp load_loop
  if read_bits(1)=0 then error        ;end bit (must be 1)
  ret

The best values for the huffman trees that I've found are 6,9,12-bit
displacements for method 0 (best for NES/Z80 code), and two less for method 1,
ie. 4,7,10-bit (best for GBA code). And 2,4,10-bit for the length values. The
smallest value in node 0, and the other values in node 10 and 11.

Notes

The decompression works similar to the GBA BIOS'es LZ77 decompression function,
but without using fixed bit-widths of length=4bit and displacement=12bit,
instead, the bit-widths are read from huffman trees (which can also define
fixed bit-widths; if data is located directly in the root node).

Unlike the GBA BIOS'es Huffman decompression function, the trees are starting
with data entries, end are ending with the root entry. The above load function
deciphers the data, and returns the root index.

With the variable bit-widths, the VPK compression rate is quite good, only,
it's a pity that the length/disp values are zero-based, eg. for 2bit and 4bit
lengths, it'd be much better to assign 2bit as 2..5, and 4bit as 6..21.

Non-VPK

The e-Reader additionally supports an alternate decompression function,
indicated by the absence of the "vpk0" ID, which supports compression of
increasing byte-values, which isn't useful for program code.

Bit15 of the VPK Size value seems to disable (de-)compression, the VPK Data
field is then containing plain uncompressed data.
```
