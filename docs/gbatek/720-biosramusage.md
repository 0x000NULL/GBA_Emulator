# BIOS RAM Usage

Source: https://problemkaputt.de/gbatek.htm#biosramusage

```
Below contains info about RAM contents at cartridge boot time (as initialized
by the BIOS/Firmware), plus info about RAM locations used by IRQ handlers and
SWI functions.

GBA BIOS RAM Usage

Below memory at 3007Fxxh is often accessed directly, or via mirrors at
3FFFFxxh.

  3000000h 7F00h User Memory and User Stack              (sp_usr=3007F00h)
  3007F00h A0h   Default Interrupt Stack (6 words/time)  (sp_irq=3007FA0h)
  3007FA0h 40h   Default Supervisor Stack (4 words/time) (sp_svc=3007FE0h)
  3007FE0h 10h   Debug Exception Stack (4 words/time)    (sp_xxx=3007FF0h)
  3007FF0h 4     Pointer to Sound Buffer (for SWI Sound functions)
  3007FF4h 3     Reserved (unused)
  3007FF7h 1     Reserved (intro/nintendo logo related)
  3007FF8h 2     IRQ IF Check Flags (for SWI IntrWait/VBlankIntrWait functions)
  3007FFAh 1     Soft Reset Re-entry Flag (for SWI SoftReset function)
  3007FFBh 1     Reserved (intro/multiboot slave related)
  3007FFCh 4     Pointer to user IRQ handler (to 32bit ARM code)

NDS BIOS RAM Usage

Below memory at 27FFxxxh is mirrored to 23FFxxxh (on retail consoles with 4MB
RAM), however, it should be accessed via address 27FFxxxh (for compatibility
with debug consoles with 8MB RAM). Accessing it via mirrors at 2FFFxxxh is also
valid (this is done by DSi enhanced games; even when running in non-DSi mode;
this allows DSi games to use the same memory addresses in NDS and DSi mode).

  2000000h ...   ARM7 and ARM9 bootcode can be loaded here (2000000h..23BFDFFh)
  2400000h ...   Debug bootcode can be loaded here (2400000h..27BFDFFh)
  23FEE00h 168h  Fragments of NDS9 firmware boot code
  27FF800h 4     NDS Gamecart Chip ID 1
  27FF804h 4     NDS Gamecart Chip ID 2
  27FF808h 2     NDS Cart Header CRC (verified)            ;hdr[15Eh]
  27FF80Ah 2     NDS Cart Secure Area CRC (not verified ?) ;hdr[06Ch]
  27FF80Ch 2     NDS Cart Missing/Bad CRC (0=Okay, 1=Missing/Bad)
  27FF80Eh 2     NDS Cart Secure Area Bad (0=Okay, 1=Bad)
  27FF810h 2     Boot handler task number (usually FFFFh at cart boot time)
  27FF812h 2     Secure disable (0=Normal, 1=Disable; Cart[078h]=BIOS[1088h])
  27FF814h 2     SIO Debug Connection Exists (0=No, 1=Yes)
  27FF816h 2     RTC Status?                 (0=Okay, 1=Bad)
  27FF818h 1     Random RTC    ;random LSB from SIO debug detect handshake
  27FF819h 37h   Zerofilled by firmware
  27FF850h 2     NDS7 BIOS CRC (5835h)
  27FF860h 4     Somewhat copy of Cart[038h], nds7 ram addr (?)
  27FF864h 4     Wifi FLASH User Settings Bad (0=Okay, 1=Bad)
  27FF868h 4     Wifi FLASH User Settings FLASH Address (fmw[20h]*8)
                   maybe recommended to use above RAM cell instead FLASH entry?
  27FF86Ch 4     Whatever (seems to be zero at cart boot time)
  27FF870h 4     Whatever (seems to be zero at cart boot time)
  27FF874h 2     Wifi FLASH firmware part5 crc16 (359Ah) (fmw[026h])
  27FF876h 2     Wifi FLASH firmware part3/part4 crc16 (fmw[004h] or ZERO)
                   Above is usually ZERO at cart boot (set to fmw[004h] only
                   when running pictochat, or maybe also when changing user
                   settings)
  27FF878h 08h   Not used
  27FF880h 4     Message from NDS9 to NDS7  (=7 at cart boot time)
  27FF884h 4     NDS7 Boot Task (also checked by NDS9) (=6 at cart boot time)
  27FF888h ..    Whatever (seems to be zero at cart boot time)
  27FF890h 4     Somewhat boot flags (somewhat B0002A22h)
                   bit10 part3/part4 loaded/decoded (bit3 set if bad crc)
                   bit28 part5 loaded/decoded with good crc
  27FF894h 36Ch  Not used (zero)
  27FFC00h 4     NDS Gamecart Chip ID 1   (copy of 27FF800h)
  27FFC04h 4     NDS Gamecart Chip ID 2   (copy of 27FF804h)
  27FFC08h 2     NDS Cart Header CRC      (copy of 27FF808h)
  27FFC0Ah 2     NDS Cart Secure Area CRC (copy of 27FF80Ah)
  27FFC0Ch 2     NDS Cart Missing/Bad CRC (copy of 27FF80Ch)
  27FFC0Eh 2     NDS Cart Secure Area Bad (copy of 27FF80Eh)
  27FFC10h 2     NDS7 BIOS CRC (5835h)    (copy of <27FF850h>)
  27FFC12h 2     Secure Disable           (copy of 27FF812h)
  27FFC14h 2     SIO Debug Exist          (copy of 27FF814h)
  27FFC16h 1     RTC Status?              (<8bit> copy of 27FF816h)
  27FFC17h 1     Random 8bit              (copy of <27FF818h>)
  27FFC18h 18h   Not used (zero)
  27FFC30h 2     GBA Cartridge Header[BEh], Reserved
  27FFC32h 3     GBA Cartridge Header[B5h..B7h], Reserved
  27FFC35h 1     Whatever flags ?
  27FFC36h 2     GBA Cartridge Header[B0h], Maker Code
  27FFC38h 4     GBA Cartridge Header[ACh], Gamecode
  27FFC3Ch 4     Frame Counter (eg. 00000332h in no$gba with original firmware)
  27FFC40h 2     Boot Indicator (1=normal; required for some NDS games, 2=wifi)
  27FFC42h 3Ch   Not used (zero) (or boot info if booted via DS Download Play)
  27FFC7Eh 2     Not used (zero)
  27FFC80h 70h   Wifi FLASH User Settings (fmw[newest_user_settings])
  27FFCF0h 10h   Not used (zero)
  27FFDxxh ..    NDS9 Debug Exception Stack (stacktop=27FFD9Ch)
  27FFD9Ch 4     NDS9 Debug Exception Vector (0=None)
  27FFDA0h ..    ...
  27FFE00h 170h  NDS Cart Header at 27FFE00h+0..16Fh
  27FFF70h ..    Not used (zerofilled at cart boot time)
  27FFFF8h 2     NDS9 Scratch addr for SWI IsDebugger check
  27FFFFAh 2     NDS7 Scratch addr for SWI IsDebugger check
  27FFFFCh ..    ...
  27FFFFEh 2     Main Memory Control (on-chip power-down I/O port)
  DTCM+3FF8h 4   NDS9 IRQ IF Check Bits (hardcoded RAM address)
  DTCM+3FFCh 4   NDS9 IRQ Handler (hardcoded RAM address)
  37F8000h FE00h ARM7 bootcode can be loaded here (37F8000h..3807DFFh)
  380F700h 1D4h  Fragments of NDS7 firmware boot code
  380F980h 4     Unknown/garbage (set to FBDD37BBh, purpose unknown)
                   NOTE: Cooking Coach is doing similar crap at 37FCF1Ch ?!?!
  380FFC0h 4     DSi7 IRQ IF2 Check Bits (hardcoded RAM address) (DSi only)
  380FFDCh ..    NDS7 Debug Stacktop / Debug Vector (0=None)
  380FFF8h 4     NDS7 IRQ IF Check Bits (hardcoded RAM address)
  380FFFCh 4     NDS7 IRQ Handler (hardcoded RAM address)
  ---
  summary of nds memory used at cartridge boot time:
  (all other memory zero-filled unless containing cartridge data)
  37F8000h..3807E00h  ;cartridge area (nds7 only)
  2000000h..23BFE00h  ;cartridge area (nds9 and nds7)
  2400000h..27BFE00h  ;cartridge area (debug ver)
  23FEE00h..23FEF68h  ;fragments of NDS9 firmware boot code
  27FF800h..27FF85Fh  ;various values (from BIOS boot code)
  27FF860h..27FF893h  ;various values (from Firmware boot code)
  27FFC00h..27FFC41h  ;various values (from Firmware boot code)
  27FFC80h..27FFCE6h  ;firmware user settings
  27FFE00h..27FFF6Fh  ;cart header
  380F700h..380F8D4h  ;fragments of NDS7 firmware boot code
  380F980h            ;set to FBDD37BBh
  ---
  register settings at cartridge boot time:
  nds9 r0..r11     = zero
  nds9 r12,r14,r15 = entrypoint
  nds9 r13         = 3002F7Ch (!)
  nds9 r13_irq     = 3003F80h
  nds9 r13_svc     = 3003FC0h
  nds9 r14/spsr_irq= zero
  nds9 r14/spsr_svc= zero
  ---
  nds7 r0..r11     = zero
  nds7 r12,r14,r15 = entrypoint
  nds7 r13         = 380FD80h
  nds7 r13_irq     = 380FF80h
  nds7 r13_svc     = 380FFC0h
  nds7 r14/spsr_irq= zero
  nds7 r14/spsr_svc= zero
  ---
  Observe that SWI SoftReset applies different stack pointers:
  Host  sp_svc    sp_irq    sp_sys    zerofilled area       return address
  NDS7  380FFDCh  380FFB0h  380FF00h  [380FE00h..380FFFFh]  Addr[27FFE34h]
  NDS9  0803FC0h  0803FA0h  0803EC0h  [DTCM+3E00h..3FFFh]   Addr[27FFE24h]

DSi BIOS RAM

  2000000h 8     AutoParam Old Title ID (former title)    ;carthdr[230h]
  2000008h 1     AutoParam Unknown/Unused
  2000009h 1     AutoParam Flags (03h=Stuff is used?)
  200000Ah 2     AutoParam Old Maker code                 ;carthdr[010h]
  200000Ch 2     AutoParam Unknown (02ECh) ;\counter/length/indices/whatever?
  200000Eh 2     AutoParam Unknown (0000h) ;/
  2000010h 2     AutoParam CRC16 on [000h..2FFh], initial=FFFFh, [010h]=0000h
  2000012h 2     AutoParam Unknown/Unused (000Fh = want Internet Settings?)
  2000014h 2ECh  AutoParam Unknown... some buffer... string maybe?
  2000300h 4     AutoLoad ID ("TLNC") (also requires BPTWL[70h]=01h)
  2000304h 1     AutoLoad Unknown/unused (usually 01h)
  2000305h 1     AutoLoad Len of data at 2000308h (01h..18h, for CRC, 18h=norm)
  2000306h 2     AutoLoad CRC16 of data at 2000308h (with initial value FFFFh)
  2000308h 8     AutoLoad Old Title ID (former title) (can be 0=anonymous)
  2000310h 8     AutoLoad New Title ID (new title to be started, 0=none)
  2000318h 4     AutoLoad Flags (bit0,1-3,4,5,6,7) ;usually 16bit, once 32bit
  200031Ch 4     AutoLoad Unused (but within checksummed area when CRC len=18h)
  2000320h E0h   AutoLoad Unused (but zerofilled upon erasing autoload area)
  2000400h 128h  System Settings from TWLCFGn.dat file (bytes 088h..1AFh)
  20005E0h 1     WlFirm Type (1=DWM-W015, 2=W024, 3=W028)    ;wifi_flash[1FDh]
  20005E1h 1     WlFirm Unknown (zero)
  20005E2h 2     WlFirm CRC16 with initial value FFFFh on [20005E4h..20005EFh]
  20005E4h 4     WlFirm RAM vars (500400h)  ;\
  20005E8h 4     WlFirm RAM base (500000h)  ; as from "Wifi Firmware" file
  20005ECh 4     WlFirm RAM size (02E000h)  ;/
  20005F0h 10h   WlFirm Unknown (zero)
  2000600h 14h   Hexvalues from HWINFO_N.dat
  2000800h ...   Unlaunch Auto-load feature (via "device:/Path/Filename.ext")
  23FEE00h 200h  DSi9 bootstrap relict
  ---
  2FEE120h 4     "nand"   <--- passed as so to launcher
  2FF80xxh
  2FF82xxh
  2FF83xxh
  2FF89xxh
  2FF8Axxh
  2FF8Bxxh
  2FF8Cxxh
  2FF8Dxxh       ... Wifi MAC address, channel mask, etc.
  2FF8Fxxh
  2FF90xxh
  2FF91xxh
  2FF9208h       FBDD37BBh (that odd "garbage" value occurs also on NDS)
  2FFA1xxh
  2FFA2xxh
  2FFA5xxh
  2FFA6xxh
  2FFA680h 12    02FD4D80h,00000000h,00001980h
  2FFA68Ch ..    Zerofilled
  ---
  2FFC000h 1000h Full Cart Header (as at 2FFE000h, but, FOR NDS ROM CARTRIDGE)
  2FFD000h 7B0h  Zerofilled
  2FFD7B0h 8+1   Version Data Filename (eg. 30,30,30,30,30,30,30,34,00)
  2FFD7B9h 1     Version Data Region   (eg. 50h="P"=Europe)
  2FFD7BAh 1     Unknown  (00)   ;bit0 = warmboot-flag-related
  2FFD7BBh 1     Unknown  (00)
  2FFD7BCh 15+1  eMMC CID (dd,ss,ss,ss,ss,03,4D,30,30,46,50,41,00,00,15), 00
  2FFD7CCh 15+1  eMMC CSD (40,40,96,E9,7F,DB,F6,DF,01,59,0F,2A,01,26,90), 00
  2FFD7DCh 4     eMMC OCR (80,80,FF,80)                                  ;20h
  2FFD7E0h 8     eMMC SCR (00,04,00,00,00,00,00,00) (for MMC: dummy/4bit);24h
  2FFD7E8h 2     eMMC RCA (01,00)                                        ;2Ch
  2FFD7EAh 2     eMMC Typ (01,00) (0=SD Card, 1=MMC Card)                ;2Eh
  2FFD7ECh 2     eMMC HCS (00,00) ;copy of OCR.bit30 (sector addressing) ;30h
  2FFD7EEh 2     eMMC ?   (00,00)                                        ;32h
  2FFD7F0h 4     eMMC ?   (00,00,00,00)                                  ;34h
  2FFD7F4h 4     eMMC CSR (00,09,00,00)   ;card status (state=tran)      ;38h
  2FFD7F8h 2     eMMC Port 4004824h setting  (00,01)  ;SD_CARD_CLK_CTL   ;3Ch
  2FFD7FAh 2     eMMC Port 4004828h setting  (E0,40)  ;SD_CARD_OPTION    ;3Eh
  2FFD7FCh 2     eMMC ?   (00,00)                                        ;40h
  2FFD7FEh 2     eMMC Device (usually 0001h=eMMC) (0000h=SD/MMC Slot?)   ;42h
  2FFD800h 1     Titles: Number of titles in below lists (max 76h)
  2FFD801h 0Fh   Titles: Zerofilled
  2FFD810h 10h   Titles: Pub Flags (1bit each) ;same maker plus public.sav
  2FFD820h 10h   Titles: Prv Flags (1bit each) ;same maker plus private.sav
  2FFD830h 10h   Titles: Jmp Flags (1bit each) ;jumpable or current-title
  2FFD840h 10h   Titles: Mkr Flags (1bit each) ;same maker
  2FFD850h 3B0h  Titles: Title IDs (8 bytes each)
  2FFDC00h 400h  Zerofilled
  2FFE000h 1000h DSi Full Cart Header (additionally to short headers)
  2FFF000h 0Ch   Zerofilled
  2FFF00Ch 4     ? 0000007Fh
  2FFF010h 4     ? 550E25B8h
  2FFF014h 4     ? 02FF4000h
  2FFF018h A68h  Zerofilled
  2FFFA80h 160h  Short Cart header (as at 2FFFE00h, but, FOR NDS ROM CARTRIDGE)
  2FFFBE0h 20h   Zerofilled

Below resembles NDS area at 27FFC00h (with added/removed stuff)...

  2FFFC00h 4     NDS Gamecart Chip ID
  2FFFC04h 20h   Zerofilled
  2FFFC24h 5     ? (04 00 73 01 03)
  2FFFC29h 7     Zerofilled
  2FFFC30h 12    GBA Cartridge Header (FF FF FF FF FF 00 FF FF FF FF FF FF)
  2FFFC3Ch 4     Frame Counter maybe? (eg. 1F 01 00 00 in cooking coach)
  2FFFC40h 2     Boot Indicator (1=ROM Cartridge,2=Wifi/tmp?,3=SD/MMC DSiware)
  2FFFC42h 3Ch   Not used (zero) (or boot info if booted via DS Download Play)
  2FFFC7Eh 2     Not used (zero)
  2FFFC80h 70h   Wifi FLASH User Settings (fmw[newest_user_settings])
  2FFFCF0h 4     ?           (3D 00 01 6E) (update counter and crc16 ?)
  2FFFCF4h 6     Wifi MAC Address (00 23 CC xx xx xx)              (fmw[036h])
  2FFFCFAh 2     Wifi Channels (usually 1041h = ch1+7+13) (based on fmw[03Ch])
  2FFFCFCh 4     Zero
  2FFFD00h 68h   Zerofilled
  2FFFD68h 4     Bitmask for Supported Languages (3Eh for Europe);\
  2FFFD6Ch 4     Unknown (00,00,00,00)                           ; from
  2FFFD70h 1     Console Region (0=JP,1=US,2=EU,3=AU,4=CHN,5=KOR); HWINFO_S.dat
  2FFFD71h 12    Serial/Barcode (ASCII, 11-12 characters)        ;
  2FFFD7Dh 3     ? (00 00 3C)                                    ;/
  2FFFD80h 0Ch   Zerofilled
  2FFFD8Ch 10h   ARM9 debug exception stack (stacktop 2FFFD9Ch)
  2FFFD9Ch 4     ARM9 debug exception vector (020D3E64h)
  2FFFDA0h 4     02F80000h                                ;\
  2FFFDA4h 4     02FFA674h                                ;
  2FFFDA8h 4     00000000h zero                           ; start addresses?
  2FFFDACh 4     01FF86E0h itcm?                          ;
  2FFFDB0h 4     027C00C0h                                ;
  2FFFDB4h 4     02FFF000h                                ;
  2FFFDB8h 4     03040000h wram?                          ;
  2FFFDBCh 4     03800000h wram?                          ;
  2FFFDC0h 4     0380C3B4h wram?                          ;/
  2FFFDC4h 4     02F80000h                                ;\
  2FFFDC8h 4     02FFC000h ptr to DSi Full Cart Header    ;
  2FFFDCCh 4     00000000h zero                           ; end addresses?
  2FFFDD0h 4     02000000h ram bottom?                    ; (for above nine
  2FFFDD4h 4     027C0780h                                ; start addresses)
  2FFFDD8h 4     02FFF680h                                ;
  2FFFDDCh 4     03040000h wram?                          ;
  2FFFDE0h 4     03800000h wram?                          ;
  2FFFDE4h 4     0380F780h wram?                          ;/
  2FFFDE8h 4     RTC Date at Boot (BCD) (yy,mm,dd,XX) (XX=maybe day-of-week?)
  2FFFDECh 4     RTC Time at Boot (BCD) (hh,ss,mm,0) (hh.bit6=maybe PM or 24h?)
  2FFFDF0h 4     Initial ARM7 Port 4004008h bits (13FBFB06h) (SCFG_EXT7)
  2FFFDF4h 1     Initial ARM7 Port 40040xxh bits (C4h)       (SCFG_xxx)
  2FFFDF5h 1     Initial ARM7 Port 400400xh bits (F0h)       (SCFG_xxx)
  2FFFDF6h 2+2   Zerofilled
  2FFFDFAh 1     Warmboot Flag (bptwl[70h] OR 80h, ie. 80h=cold or 81h=warm)
  2FFFDFBh 1     01h
  2FFFDFCh 4     Pointer to TWLCFGn.dat (usually 2000400h) (or 0=2000400h)
  2FFFE00h 160h  Short Cart header (unlike NDS, only 160h, not 170h)
  2FFFF60h A0h   Zerofilled
 37FA414h       "nand:/title/....app"  <-- [1D4h]+3C0h  (without Device List!)
 380C400h 22E4h BIOS Keys (as from Boot Stage 1, see there)
  380F010h 10h   AES key for dev.kp (E5,CC,5A,8B,...) (optional/for launcher)
  380F600h 200h  DSi7 bootstrap relict (at 3FFF600h aka mirrored to 380F600h)
  380FFC0h 4     DSi7 IRQ IF2 Check Bits (hardcoded RAM address) (DSi only)
  380FFC4h 4     DSi7 SCFG_EXT7 setting
  380FFC8h 2     DSi7 SCFG_misc bits
  380FFDCh ..    DSi7 Debug Stacktop / Debug Vector (0=None)
  380FFF8h 4     DSi7 IRQ IF Check Bits (hardcoded RAM address)
  380FFFCh 4     DSi7 IRQ Handler (hardcoded RAM address)
  xxxxxxxh ?     ARM7i and ARM9 bootcode can be loaded WHERE and WHERE?
  cart_header[1D4h] 400h SD/MMC Device List ARM7 RAM; initialized by firmware

Initial state after DSi BIOS ROM bootcode (when starting eMMC bootcode)
requires only a few memory blocks in ITCM, ARM7 WRAM, and AES keyslots:

  1FFC400h 400h  BIOS Keys from FFFF87F4h (C3 02 93 DE ..) RSA keys (8x80h)
  1FFC800h 80h   BIOS Keys from FFFF9920h (30 33 26 D5 ..) Whatever
  1FFC880h 14h   Whatever, should/may be zerofilled?
  1FFC894h 1048h BIOS Keys from FFFF99A0h (99 D5 20 5F ..) Blowfish/NDS-mode
  1FFD8DCh 1048h BIOS Keys from FFFFA9E8h (D8 18 FA BF ..) Blowfish/unused?
  3FFC400h 200h  BIOS Keys from 00008188h (CA 13 31 79 ..) Whatever, 32x10h AES?
  3FFC600h 40h   BIOS Keys from 0000B5D8h (AF 1B F5 16 ..) Whatever, AES?
  3FFC640h 14h   Whatever, must be zerofilled
  3FFC654h 1048h BIOS Keys from 0000C6D0h (59 AA 56 8E ..) Blowfish/DSi-mode
  3FFD69Ch 1048h BIOS Keys from 0000D718h (54 86 13 3B ..) Blowfish/unused?
  3FFE6E4h 44h   eMMC Info (to be relocated to 2FFD7BCh, see there for details)
  4004450h 8     AES Key0.X ("Nintendo") for modcrypt
  4004480h 10h   AES Key1.X (CPU/Console ID and constants) for dev.kp and Tad
  40044B0h 10h   AES Key2.X ("Nintendo DS",...)            for Tad
  40044E0h 1Ch   AES Key3.X/Y (CPU/Console ID and constants) for eMMC
  2000000h ...   Warmboot Param (optional, passed on to New Title)
  2000300h 20h   Warmboot Info  (optional, passed on to Launcher)

3DS Initial RAM content (from bootrom's FIRM loader)

  01FFB800h 90h    ARM9 ITCM+3800h Decrypted OTP
  01FFB900h 200h   ARM9 ITCM+3900h NCSD header
  01FFBB00h 200h   ARM9 ITCM+3B00h FIRM header
  01FFBD00h 4*100h ARM9 ITCM+3D00h rsa keys ? private key 0..3 (modulo)
  01FFC100h 4*200h ARM9 ITCM+4100h rsa keys ? private key 4..7 (modulo+exp)
  01FFC900h 400h   ARM9 ITCM+4900h ??? ... copied from 1FFFD000h or 1FFFD400h
  01FFCD00h 40h    ARM9 ITCM+4D00h keys?
  01FFCD40h 40h    ARM9 ITCM+4D40h keys?
  01FFCD80h 64h    ARM9 ITCM+4D80h sdmmc info?
  01FFD000h 2470h  ARM9 ITCM+5000h Bootrom DSi Keys (RSA/AES/Blowfish)
  08000000h 30h    ARM9 Exception Vectors (ldr pc,[$+4] opcodes)
  1FFFFFA0h 30h    ARM11 Exception Vectors (ldr pc,[$+4] opcodes)
  1FFFFFDCh 4      ARM11 Core 1 Entrypoint (32bit word)
  20000000h 1000h  FCRAM FIRM Launch Params (on warmboot only)
```
