# GBA Flashcards

Source: https://problemkaputt.de/gbatek.htm#gbaflashcards

```
Flashcards are re-writable cartridges using FLASH memory, allowing to test even
multiboot-incompatible GBA software on real hardware, providing a good
development environment when used in combination with a reasonable software
debugger.

The carts can be written to from external tools, or directly from GBA programs.

Below are pseudo code flowcharts for detect, erase, and write operations.

All flash reads/writes are meant to be 16bit (ldrh/strh) memory accesses.

detect_flashcard:

 configure_flashcard(9E2468Ah,9413h)    ;unlock flash advance cards
 turbo=1, send_command(8000000h,90h)    ;enter ID mode (both chips, if any)
 maker=[8000000h], device=[8000000h+2]
 IF maker=device THEN device=[8000000h+4] ELSE turbo=0
 flashcard_read_mode                    ;exit ID mode
 search (maker+device*10000h) in device_list
 total/erase/write_block_size = list_entry SHL turbo

flashcard_erase(dest,len):

 FOR x=1 to len/erase_block_size
  send_command(dest,20h)        ;erase sector command
  send_command(dest,D0h)        ;confirm erase sector
  dest=dest+erase_block_size
 IF wait_busy=okay THEN NEXT x
 enter_read_mode                ;exit erase/status mode

flashcard_write(src,dest,len):

 siz=write_block_size
 FOR x=1 to len/siz
  IF siz=2 THEN send_command(dest,10h)  ;write halfword command
  IF siz>2 THEN send_command(dest,E8h)  ;write to buffer command
  IF siz>2 THEN send_command(dest,16-1) ;buffer size 16 halfwords (per chip)
  FOR y=1 TO siz/2
   [dest]=[src], dest=dest+2, src=src+2 ;write data to buffer
  NEXT y
  IF siz>2 THEN send_command(dest,D0h)  ;confirm write to buffer
 IF wait_busy=okay THEN NEXT x
 enter_read_mode                        ;exit write/status mode

send_command(adr,val):

 [adr]=val
 IF turbo THEN [adr+2]=val

enter_read_mode:

 send_command(8000000h,FFh)     ;exit status mode
 send_command(8000000h,FFh)     ;again maybe more stable (as in jeff's source)

flashcard_wait_busy:

 start=time
 REPEAT
  stat=[8000000h] XOR 80h
  IF turbo THEN stat=stat OR ([8000000h+2] XOR 80h)
  IF (stat AND 7Fh)>0 THEN error
  IF (stat AND 80h)=0 THEN ready
  IF time-start>5secs THEN timeout
 UNTIL ready OR error OR timeout
 IF error OR timeout THEN send_command(8000000h,50h)    ;clear status

configure_flashcard(adr,val):   ;required for Flash Advance cards only

 [930ECA8h]=5354h
 [802468Ah]=1234h, repeated 500 times
 [800ECA8h]=5354h
 [802468Ah]=5354h
 [802468Ah]=5678h, repeated 500 times
 [930ECA8h]=5354h
 [802468Ah]=5354h
 [8ECA800h]=5678h
 [80268A0h]=1234h
 [802468Ah]=ABCDh, repeated 500 times
 [930ECA8h]=5354h
 [adr]=val

init_backup:  ;no info how to use that exactly

 configure_flashcard(942468Ah,???)

device_list:  (id code, total/erase/write sizes in bytes)

  ID Code    Total   Erase  Write  Name
  -??-00DCh      ?       ?      ?  Hudson Cart (???)
  00160089h     4M    128K     32  Intel i28F320J3A (Flash Advance)
  00170089h     8M    128K     32  Intel i28F640J3A (Flash Advance)
  00180089h    16M    128K     32  Intel i28F128J3A (Flash Advance)
  00E200B0h      ?     64K      2  Sharp LH28F320BJE ? (Nintendo)

Notes

All flashcards should work at 4,2 waitstates (power on default), most
commercial games change waits to 3,1 which may work unstable with some/older FA
flashcards. Intel FLASH specified to have a lifetime of 100,000 erases, and
average block erase time 1 second (up to 5 second in worst cases).

Aside from the main FLASH memory, Flash Advance (FA) (aka Visoly) cards
additionally contain battery buffered SRAM backup, and FLASH backup, and in
some cases also EEPROM backup.

Turbo FA cards are containing two chips interlaced (at odd/even halfword
addresses), allowing to write/erase both chips simultaneously, resulting in
twice as fast programming time.

Standard Nintendo flash carts have to be modified before you can actually write
to them. This is done by removing resistor R7 and putting it at empty location
R8.

Mind that write/erase/detect modes output status information in ROM area, so
that in that modes all GBA program code (and any interrupt handlers) must be
executed in WRAM, not in ROM.

Thanks to Jeff Frohwein for his FAQ and CARTLIB sample in FLGBA at devrs.com
```
