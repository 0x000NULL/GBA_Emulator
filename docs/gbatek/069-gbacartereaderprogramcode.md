# GBA Cart e-Reader Program Code

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderprogramcode

```
The GBA/Z80/NES program code is stored in the VPK compressed area.

NES-type is indicated by header [2Ah].Bit2, GBA-type is indicated by the NULL
value inserted between VPK Size and VPK Data, otherwise Z80-type is used.

GBA Format

Load Address and Entrypoint are at 2000000h (in ARM state). The 32bit word at
2000008h is eventually destroyed by the e-Reader. Namely,

  IF e-Reader is Non-Japanese,
  AND [2000008h] is outside of range of 2000000h..20000E3h,
  AND only if booted from camera (not when booted from FLASH?),
  THEN [2000008h]=[2000008h]-0001610Ch ELSE [2000008h] kept intact

Existing multiboot-able GBA binaries can be converted to e-Reader format by,

  Store "B 20000C0h" at 2000000h   ;redirect to RAM-entrypoint
  Zerofill 2000004h..20000BFh      ;erase header (for better compression rate)
  Store 01h,01h at 20000C4h        ;indicate RAM boot

The GBA code has full access to the GBA hardware, and may additionally use
whatever API functions contained in the e-Reader BIOS. With the incoming LR
register value, "mov r0,N, bx lr" returns to the e-Reader BIOS (with N being
0=Restart, or 2=To_Menu). No idea if it's necessary to preserve portions of RAM
when returning to the e-Reader BIOS?

Caution: Unlike for normal GBA cartridges/multiboot files, the hardware is left
uninitialized when booting dotcodes (among others: sound DMA is active, and
brightness is set to zero), use "mov r0,0feh, swi 010000h" to get the normal
settings.

NES Format

Emulates a NES (Nintendo Entertainment System) console (aka Family Computer).

The visible 240x224 pixel NES/NTSC screen resolution is resampled to 240x160 to
match the smaller vertical resolution of the GBA hardware. So, writing e-Reader
games in NES format will result in blurred screen output. The
screen/sound/joypad is accessed via emulated NES I/O ports, program code is
running on an emulated 6502 8bit CPU, for more info on the NES hardware, see
no$nes debugger specifications, or

  http://problemkaputt.de/everynes.htm

The e-Reader's NES emulator supports only 16K PRG ROM, followed by 8K VROM. The
emulation accuracy is very low, barely working with some of Nintendo's own NES
titles; running the no$nes diagnostics program on it has successfully failed on
ALL hardware tests ;-)

The load address for the 16K PRG-ROM is C000h, the 16bit NMI vector at [FFFAh]
is encrypted like so:

  for i=17h to 0
   for j=07h to 0, nmi = nmi shr 1, if carry then nmi = nmi xor 8646h, next j
   nmi = nmi xor (byte[dmca_data+i] shl 8)
  next i
  dmca_data: db 0,0,'DMCA NINTENDO E-READER'

The 16bit reset vector at [FFFCh] contains:

  Bit0-14  Lower bits of Entrypoint (0..7FFFh = Address 8000h..FFFFh)
  Bit15    Nametable Mode (0=Vertical Mirroring, 1=Horizontal Mirroring)

reportedly,

   (NES limitations, 1 16K program rom + 1-2 8K CHR rom, mapper 0 and 1)
   ines mapper 1 would be MMC1, rather than CNROM (ines mapper 3)?
   but, there are more or less NONE games that have 16K PRG ROM + 16K VROM?

The L+R Button key-combination allows to reset the NES, however, there seems to
be no way to return to the e-Reader BIOS.

Z80/8080 Format

The e-Reader doesn't support the following Z80 opcodes:

  CB [Prefix]     E0 RET PO   E2 JP PO,nn   E4 CALL PO,nn   27 DAA    76 HALT
  ED [Prefix]     E8 RET PE   EA JP PE,nn   EC CALL PE,nn   D3 OUT (n),A
  DD [IX Prefix]  F3 DI       08 EX AF,AF'  F4 CALL P,nn    DB IN A,(n)
  FD [IY Prefix]  FB EI       D9 EXX        FC CALL M,nn    xx RST 00h..38h

That is leaving not more than six supported Z80 opcodes (DJNZ, JR, JR
c/nc/z/nz), everything else are 8080 opcodes. Custom opcodes are:

  76 WAIT A frames, D3 WAIT n frames, and C7/CF RST 0/8 used for API calls.

The load address and entrypoint are at 0100h in the emulated Z80 address space.
The Z80 doesn't have direct access to the GBA hardware, instead
video/sound/joypad are accessed via API functions, invoked via RST 0 and RST 8
opcodes, followed by an 8bit data byte, and with parameters in the Z80 CPU
registers. For example, "ld a,02h, rst 8, db 00h" does return to the e-Reader
BIOS.

The Z80/8080 emulation is incredibly inefficient, written in HLL code,
developed by somebody whom knew nothing about emulation nor about ARM nor about
Z80/8080 processors.

Running GBA-code on Japanese/Original e-Reader

Original e-Reader supports Z80 code only, but can be tweaked to run GBA-code:

  retry:
   ld bc,data // ld hl,00c8h      ;src/dst
  lop:
   ld a,[bc] // inc bc // ld e,a  ;lsb
   ld a,[bc] // inc bc // ld d,a  ;msb
   dw 0bcfh ;aka rst 8 // db 0bh  ;[4000000h+hl]=de (DMA registers)
   inc hl // inc  hl // ld a,l
   cp a,0dch // jr nz,lop
  mod1 equ $+1
   dw 37cfh ;aka rst 8 // db 37h  ;bx 3E700F0h
  ;below executed only on jap/plus... on jap/plus, above 37cfh is hl=[400010Ch]
   ld a,3Ah // ld [mod1],a                  ;bx 3E700F0h (3Ah instead 37h)
   ld hl,1 // ld [mod2],hl // ld [mod3],hl  ;base (0200010Ch instead 0201610Ch)
   jr retry
  data:
  mod2 equ $+1
   dd loader         ;40000C8h dma2sad (loader)            ;\
   dd 030000F0h      ;40000CCh dma2dad (mirrored 3E700F0h) ; relocate loader
   dd 8000000ah      ;40000D0h dma2cnt (copy 0Ah x 16bit)  ;/
  mod3 equ $+1
   dd main           ;40000D4h dma3sad (main)              ;\prepare main reloc
   dd 02000000h      ;40000D8h dma3dad (2000000h)          ;/dma3cnt see loader
   .align 2          ;alignment for 16bit-halfword
  org $+201600ch     ;jap/plus: adjusted to org $+200000ch
  loader:
   mov r0,80000000h  ;(dma3cnt, copy 10000h x 16bit)
   mov r1,04000000h  ;i/o base
   strb r1,[r1,208h] ;ime=0 (better disable ime before moving ram)
   str r0,[r1,0DCh]  ;dma3cnt (relocate to 2000000h)
   mov r15,2000000h  ;start relocated code at 2000000h in ARM state
  main:
   ;...insert/append whatever ARM code here...
   end
```
