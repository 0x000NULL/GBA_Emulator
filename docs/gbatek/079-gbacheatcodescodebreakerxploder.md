# GBA Cheat Codes - Codebreaker/Xploder

Source: https://problemkaputt.de/gbatek.htm#gbacheatcodescodebreakerxploder

```
Codebreaker Codes

  0000xxxx 000y  Enable Code 1 - Game ID
  1aaaaaaa 000z  Enable Code 2 - Hook Address
  2aaaaaaa yyyy  [aaaaaaa]=[aaaaaaa] OR yyyy
  3aaaaaaa 00yy  [aaaaaaa]=yy
  4aaaaaaa yyyy  [aaaaaaa+0..(cccc-1)*ssss]=yyyy+0..(cccc-1)*ssss
  iiiicccc ssss  parameters for above code
  5aaaaaaa cccc  [aaaaaaa+0..(cccc-1)]=11,22,33,44,etc.
  11223344 5566  parameter bytes 1..6 for above code (example)
  77880000 0000  parameter bytes 7..8 for above code (padded with zero)
  6aaaaaaa yyyy  [aaaaaaa]=[aaaaaaa] AND yyyy
  7aaaaaaa yyyy  IF [aaaaaaa]=yyyy THEN (next code)
  8aaaaaaa yyyy  [aaaaaaa]=yyyy
  9xyyxxxx xxxx  Enable Code 0 - Encrypt all following codes (optional)
  Aaaaaaaa yyyy  IF [aaaaaaa]<>yyyy THEN (next code)
  Baaaaaaa yyyy  IF [aaaaaaa]>yyyy THEN (next code) (signed comparison)
  Caaaaaaa yyyy  IF [aaaaaaa]<yyyy THEN (next code) (signed comparison)
  D0000020 yyyy  IF [joypad] AND yyyy = 0 THEN (next code)
  Eaaaaaaa yyyy  [aaaaaaa]=[aaaaaaa]+yyyy
  Faaaaaaa yyyy  IF [aaaaaaa] AND yyyy THEN (next code)

Codebreaker Enable Codes

Hook Address 'aaaaaaa' is a 25bit offset in ROM-image (0-1FFFFFFh).

Flag byte 'y' (usually 0Ah), Bit1=Disable IRQs, Bit3=CRC Exists.

Code Handler Store Address 'z' (0-7, usually 7) (8000100h+z*400000h).

Checksum 'xxxx' for first 64Kbytes of cartridge (no$gba pads by FFh if ROM is
smaller than 64K). Calculated, by using unsigned 16bit values, as such:

  crc=FFFFh
  for i=0 to FFFFh
   x=byte[i] xor (crc/100h)
   x=x xor (x/10h)
   crc=(crc*100h) xor (x*1001h) xor (x*20h)
  next i

Codebreaker Encryption

codebreaker_change_encryption:

Encryption can be (optionally) activated by code "9xyyxxxx xxxx",

  for i=0 to 2Fh, swaplist[i]=i, next i
  randomizer = 1111h xor byte[code+4]                              ;LSB value
  for i=0 to 4Fh
    exchange swaplist[random MOD 30h] with swaplist[random MOD 30h]
  next i
  halfword[seedlist+0] = halfword[code+0]                          ;LSW address
  randomizer = 4EFAD1C3h
  for i=0 to byte[code+3]-91h, randomizer=random, next i           ;MSB address
  word[seedlist+2]=random, halfword[seedlist+6]=random
  randomizer = F254h xor byte[code+5]                              ;MSB value
  for i=0 to byte[code+5]-01h, randomizer=random, next i           ;MSB value
  word[seedlist+8]=random, halfword[seedlist+12]=random
  ;note: byte[code+2] = don't care
  ret

The above random function works like so:

  randomizer=randomizer*41C64E6Dh+3039h, x=(randomizer SHL 14 AND C0000000h)
  randomizer=randomizer*41C64E6Dh+3039h, x=(randomizer SHR 1  AND 3FFF8000h)+x
  randomizer=randomizer*41C64E6Dh+3039h, x=(randomizer SHR 16 AND 00007FFFh)+x
  return(x)

Once when encryption is activated, all following codes are decrypted like so:

  for i=2Fh to 0
    j=swaplist[i]
    bitno1=(i AND 7), index1=xlatlist[i/8]
    bitno2=(j AND 7), index2=xlatlist[j/8]
    exchange [code+index1].bitno1 with [code+index2].bitno2
  next i
  word[code+0] = word[code+0] xor word[seedlist+8]
  i = (byte[code+3]*1010000h + byte[code+0]*100h + byte[code+5])
  i = (halfword[code+1]*10001h) xor (word[seedlist+2]) xor i
  i = (byte[seedlist+0]*1010101h) xor (byte[seedlist+1]*1000000h) xor i
  j = (byte[code+5] + (byte[code+0] xor byte[code+4])*100h)
  j = (byte[seedlist+0]*101h) xor halfword[seedlist+6] xor j
  word[code+0] = i, halfword[code+4] = j

The above xlatlist is fixed: xlatlist[0..5] = 3,2,1,0,5,4
```
