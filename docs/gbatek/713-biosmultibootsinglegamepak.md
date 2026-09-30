# BIOS Multi Boot (Single Game Pak)

Source: https://problemkaputt.de/gbatek.htm#biosmultibootsinglegamepak

```
MultiBoot

SWI 25h (GBA) - MultiBoot

This function uploads & starts program code to slave GBAs, allowing to
launch programs on slave units even if no cartridge is inserted into the slaves
(this works because all GBA BIOSes contain built-in download procedures in
ROM).

However, the SWI 25h BIOS upload function covers only 45% of the required
Transmission Protocol, the other 55% must be coded in the master cartridge (see
Transmission Protocol below).

  r0  Pointer to MultiBootParam structure
  r1  Transfer Mode (undocumented)
       0=256KHz, 32bit, Normal mode    (fast and stable)
       1=115KHz, 16bit, MultiPlay mode (default, slow, up to three slaves)
       2=2MHz,   32bit, Normal mode    (fastest but maybe unstable)
  Note: HLL-programmers that are using the MultiBoot(param_ptr) macro cannot
  specify the transfer mode and will be forcefully using MultiPlay mode.

Return:

  r0  0=okay, 1=failed

See below for more details.

Multiboot Parameter Structure

Size of parameter structure should be 4Ch bytes (the current GBA BIOS uses only
first 44h bytes though). The following entries must be set before calling SWI
25h:

  Addr Size Name/Expl.
  14h  1    handshake_data (entry used for normal mode only)
  19h  3    client_data[1,2,3]
  1Ch  1    palette_data
  1Eh  1    client_bit (Bit 1-3 set if child 1-3 detected)
  20h  4    boot_srcp  (typically 8000000h+0C0h)
  24h  4    boot_endp  (typically 8000000h+0C0h+length)

The transfer length (excluding header data) should be a multiple of 10h,
minimum length 100h, max 3FF40h (ca. 256KBytes). Set palette_data as
"81h+color*10h+direction*8+speed*2", or as "0f1h+color*2" for fixed palette,
whereas color=0..6, speed=0..3, direction=0..1. The other entries
(handshake_data, client_data[1-3], and client_bit) must be same as specified in
Transmission Protocol (see below hh,cc,y).

Multiboot Transfer Protocol

Below describes the complete transfer protocol, normally only the Initiation
part must be programmed in the master cartridge, the main data transfer can be
then performed by calling SWI 25h, the slave program is started after SWI 25h
completion.

The ending handshake is normally not required, when using it, note that you
will need custom code in BOTH master and slave programs.

  Times  Send   Receive  Expl.
  -----------------------Required Transfer Initiation in master program
  ...    6200   FFFF     Slave not in multiplay/normal mode yet
  1      6200   0000     Slave entered correct mode now
  15     6200   720x     Repeat 15 times, if failed: delay 1/16s and restart
  1      610y   720x     Recognition okay, exchange master/slave info
  60h    xxxx   NN0x     Transfer C0h bytes header data in units of 16bits
  1      6200   000x     Transfer of header data completed
  1      620y   720x     Exchange master/slave info again
  ...    63pp   720x     Wait until all slaves reply 73cc instead 720x
  1      63pp   73cc     Send palette_data and receive client_data[1-3]
  1      64hh   73uu     Send handshake_data for final transfer completion
  -----------------------Below is SWI 25h MultiBoot handler in BIOS
  DELAY  -      -        Wait 1/16 seconds at master side
  1      llll   73rr     Send length information and receive random data[1-3]
  LEN    yyyy   nnnn     Transfer main data block in units of 16 or 32 bits
  1      0065   nnnn     Transfer of main data block completed, request CRC
  ...    0065   0074     Wait until all slaves reply 0075 instead 0074
  1      0065   0075     All slaves ready for CRC transfer
  1      0066   0075     Signalize that transfer of CRC follows
  1      zzzz   zzzz     Exchange CRC must be same for master and slaves
  -----------------------Optional Handshake (NOT part of master/slave BIOS)
  ...    ....   ....     Exchange whatever custom data

Legend for above Protocol

  y     client_bit, bit(s) 1-3 set if slave(s) 1-3 detected
  x     bit 1,2,or 3 set if slave 1,2,or 3
  xxxx  header data, transferred in 16bit (!) units (even in 32bit normal mode)
  nn    response value for header transfer, decreasing 60h..01h
  pp    palette_data
  cc    random client_data[1..3] from slave 1-3, FFh if slave not exists
  hh    handshake_data, 11h+client_data[1]+client_data[2]+client_data[3]
  uu    random data, not used, ignore this value

Below automatically calculated by SWI 25h BIOS function (don't care about)

  llll  download length/4-34h
  rr    random data from each slave for encryption, FFh if slave not exists
  yyyy  encoded data in 16bit (multiplay) or 32bit (normal mode) units
  nnnn  response value, lower 16bit of destadr in GBA memory (00C0h and up)
  zzzz  16bit download CRC value, must be same for master and slaves

Pseudo Code for SWI 25h Transfer with Checksum and Encryption calculations

  if normal_mode    then c=C387h:x=C37Bh:k=43202F2Fh
  if multiplay_mode then c=FFF8h:x=A517h:k=6465646Fh
  m=dword(pp,cc,cc,cc):f=dword(hh,rr,rr,rr)
  for ptr=000000C0h to (file_size-4) step 4
    c=c xor data[ptr]:for i=1 to 32:c=c shr 1:if carry then c=c xor x:next
    m=(6F646573h*m)+1
    send_32_or_2x16 (data[ptr] xor (-2000000h-ptr) xor m xor k)
  next
  c=c xor f:for i=1 to 32:c=c shr 1:if carry then c=c xor x:next
  wait_all_units_ready_for_checksum:send_32_or_1x16 (c)

Whereas, explained: c=chksum,x=chkxor,f=chkfin,k=keyxor,m=keymul

Multiboot Communication

In Multiplay mode, master sends 16bit data, and receives 16bit data from each
slave (or FFFFh if none). In Normal mode, master sends 32bit data (upper 16bit
zero, lower 16bit as for multiplay mode), and receives 32bit data (upper 16bit
as for multiplay mode, and lower 16bit same as lower 16bit previously sent by
master). Because SIODATA32 occupies same addresses as SIOMULTI0-1, the same
transfer code can be used for both multiplay and normal mode (in normal mode
SIOMULTI2-3 should be forced to FFFFh though). After each transfer, master
should wait for Start bit cleared in SIOCNT register, followed by a 36us delay.

Note: The multiboot slave would also recognize data being sent in Joybus mode,
however, master GBAs cannot use joybus mode (because GBA hardware cannot act as
master in joybus mode).

Multiboot Slave Header

The transferred Header block is written to 2000000-20000BFh in slave RAM, the
header must contain valid data (identically as for normal ROM-cartridge
headers, including a copy of the Nintendo logo, correct header CRC, etc.), in
most cases it'd be recommended just to transfer a copy of the master cartridges
header from 8000000h-80000BFh.

Multiboot Slave Program/Data

The transferred main program/data block is written to 20000C0h and up (max
203FFFFh) in slave RAM, note that absolute addresses in the program must be
then originated at 2000000h rather than 8000000h. In case that the master
cartridge is 256K or less, it could just transfer a copy of the whole cartridge
at 80000C0h and up, the master should then copy & execute its own ROM data
into RAM as well.

Multiboot Slave Extended Header

For Multiboot slaves, separate Entry Point(s) must be defined at the beginning
of the Program/Data block (the Entry Point in the normal header is ignored),
also some reserved bytes in this section are overwritten by the Multiboot
procedure. For more information see chapter about Cartridge Header.

Multiboot Slave with Cartridge

Beside for slaves without cartridge, multiboot can be also used for slaves
which do have a cartridge inserted, if so, SELECT and START must be kept held
down during power-on in order to switch the slave GBA into Multiboot mode (ie.
to prevent it from starting the cartridge as normally).

The general idea is to enable newer programs to link to any existing older GBA
programs, even if these older programs originally didn't have been intended to
support linking.

The uploaded program may access the slaves SRAM, Flash ROM, or EEPROM (if any,
allowing to read out or modify slave game positions), as well as cartridge ROM
at 80000A0h-8000FFFh (the first 4KBytes, excluding the nintendo logo, allowing
to read out the cartridge name from the header, for example).

The main part of the cartridge ROM is meant to be locked out in order to
prevent software pirates from uploading "intruder" programs which would send
back a copy of the whole cartridge to the master, however, for good or evil, at
present time, current GBA models and GBA carts do not seem to contain any such
protection.

Uploading Programs from PC

Beside for the ability to upload a program from one GBA to another, this
feature can be also used to upload small programs from a PC to a GBA. For more
information see chapter about External Connectors.

Nintendo DS

The GBA multiboot function requires a link port, and so, works on GBA and GBA
SP only. The Nintendo DS in GBA mode does include the multiboot BIOS function,
but it won't be of any use as the DS doesn't have a link port.
```
