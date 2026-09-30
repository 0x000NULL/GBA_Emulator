# GBA Cart e-Reader I/O Ports

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderioports

```
DF80000h Useless Register (R/W)

  0     Output to PGA.Pin93 (which seems to be not connected to anything)
  1-3   Unknown, read/write-able (not used by e-Reader BIOS)
  4-15  Always zero (0)

DFA0000h Reset Register (R/W)

  0    Always zero              (0)
  1    Reset Something?         (0=Normal, 1=Reset)
  2    Unknown, always set      (1)
  3    Unknown, read/write-able (not used by e-Reader BIOS)
  4-7  Always zero              (0)
  8    Unknown, read/write-able (not used by e-Reader BIOS)
  9-15 Always zero              (0)

DFC0000h..DFC0027h Scanline Data (R)

Scanline data (40 bytes, for 320 pixels, 1bit per pixel, 0=black, 1=white).

The first (leftmost) pixel is located in the LSB of the LAST byte.

Port E00FFB1h.Bit1 (and [4000202h].Bit13) indicates when a new scanline is
present, the data should be then transferred to RAM via DMA3 (SAD=DFC0000h,
DAD=buf+y*28h, CNT=80000014h; a slower non-DMA transfer method would result in
missed scanlines). After the DMA, software must reset E00FFB1h.Bit1.

Note: The scanning resolution is 1000 DPI.

DFC0028h+(0..2Fh*2) Brightest Pixels of 8x6 Blocks (R)

  0-6  Max Brightness (00h..7Fh; 00h=All black, 7Fh=One or more white)
  7-15 Always zero

Can be used to adjust the Port E00FF80h..E00FFAFh settings.

DFC0088h Darkest Pixel of whole Image (R)

  0-7  Max Darkness   (00h..7Fh; 00h=One or more black, 7Fh=All white)
  8-15 Always zero

Can be used to adjust the Port E00FF80h..E00FFAFh settings.

E00FF80h..E00FFAFh Intensity Boundaries for 8x6 Blocks (R/W)

The 320x246 pixel camera input is split into 8x6 blocks (40x41 pixels each),
with Block00h=Upper-right, Block07h=Upper-left, ..., Block27h=Lower-left. The
boundary values for the separate blocks are used for 128-grayscale to 2-color
conversion, probably done like "IF Pixel>Boundary THEN white ELSE black".

  0-6  Block Intensity Boundaries (0..7Fh; 7Fh=Whole block gets black)
  7    Always zero

The default boundary values are stored in FLASH memory, the values are
typically ranging from 28h (outer edges) to 34h (center image), that in respect
to the light source (the two LEDs are emitting more light to the center
region).

E00FFB0h Control Register 0 (R/W)

  0    Serial Data       (Low/High)
  1    Serial Clock      (Low/High)
  2    Serial Direction  (0=Input, 1=Output)
  3    Led/Irq Enable    (0=Off, 1=On; Enable LED and Gamepak IRQ)
  4    Start Scan        (0=Off, 1=Start) (0-to-1 --> Resync line 0)
  5    Phi 16MHz Output  (0=Off, 1=On; Enable Clock for Camera, and for LED)
  6    Power 3V Enable   (0=Off, 1=On; Enable 3V Supply for Camera)
  7    Not used          (always 0) (sometimes 1) (Read only)

E00FFB1h Control Register 1 (R/W)

  0    Not used          (always 0)
  1    Scanline Flag     (1=Scanline Received, 0=Acknowledge)
  2-3  Not used          (always 0)
  4    Strange Bit       (0=Normal, 1=Force Resync/Line0 on certain interval?)
  5    LED Anode Voltage (0=3.0V, 1=5.1V; requires E00FFB0h.Bit3+5 to be set)
  6    Not used          (always 0)
  7    Input from PGA.Pin22, always high (not used by e-Reader) (Read Only)

Bit1 can be SET by hardware only, software can only RESET that bit, the Gamepak
IRQ flag (Port 4000202h.Bit13) becomes set on 0-to-1 transitions.

E00FFB2h Light Source LED Kathode Duration (LSB) (R/W)

E00FFB3h Light Source LED Kathode Duration (MSB) (R/W)

Selects the LED Kathode=LOW Duration, aka the LED=ON Duration. That does act as
pulse width modulated LED brightness selection (the camera seems to react
slowly enough to view the light as being dimmed to medium, rather than seeing
the actual light ON and OFF states). The PWM timer seems to be clocked at 8MHz.
The hardware clips timer values 2000h..FFFFh to max 2000h (=1ms). Additionally,
the e-Reader BIOS clips values to max 11B3h. Default setting is found in FLASH
calibration data. A value of 0000h disables the LED.

Serial Port Registers (Camera Type 1) (DV488800) (calib_data[3Ch]=1)

All 16bit values are ordered MSB,LSB. All registers are whole 8bit
Read/Write-able, except 00h,57h-5Ah (read only), and 53h-55h (2bit only).

  Port     Expl.               (e-Reader Setting)
  00h      Maybe Chip ID (12h) (not used by e-Reader BIOS) (Read Only)
  01h                          (05h)    ;-Bit0: 1=auto-repeat scanning?
  02h                          (0Eh)
  10h-11h  Vertical Scroll     (calib_data[30h]+7)
  12h-13h  Horizontal Scroll   (0030h)
  14h-15h  Vertical Size       (00F6h=246)
  16h-17h  Horizontal Size     (0140h=320)
  20h-21h  H-Blank Duration    (00C4h)
  22h-23h                      (0400h)  ;-Upper-Blanking in dot-clock units?
  25h                          (var)    ;-bit1: 0=enable [57h..5Ah] ?
  26h                          (var)    ;\maybe a 16bit value
  27h                          (var)    ;/
  28h                          (00h)
  30h      Brightness/contrast (calib_data[31h]+/-nn)
  31h-33h                      (014h,014h,014h)
  34h      Brightness/contrast (02h)
  50h-52h  8bit Read/Write     (not used by e-Reader BIOS)
  53h-55h  2bit Read/Write     (not used by e-Reader BIOS)
  56h      8bit Read/Write     (not used by e-Reader BIOS)
  57h-58h  16bit value, used to autodetect/adjust register[30h] (Read Only)
  59h-5Ah  16bit value, used to autodetect/adjust register[30h] (Read Only)
  80h-FFh  Mirrors of 00h..7Fh (not used by e-Reader BIOS)

All other ports are unused, writes to those ports are ignored, and reads are
returning data mirrored from other ports; that is typically data from 2 or more
ports, ORed together.

Serial Port Registers (Camera Type 2) (calib_data[3Ch]=2)

All 16bit values are using more conventional LSB,MSB ordering, and port numbers
are arranged in a more reasonable way. The e-Reader BIOS doesn't support (or
doesn't require) brightness adjustment for this camera module.

  Port     Expl.             (e-Reader Setting)
  00h                        (22h)
  01h                        (50h)
  02h-03h  Vertical Scroll   (calib_data[30h]+28h)
  04h-05h  Horizontal Scroll (001Eh)
  06h-07h  Vertical Size     (00F6h)    ;=246
  08h-09h  Horizontal Size   (0140h)    ;=320
  0Ah-0Ch                    (not used by e-Reader BIOS)
  0Dh                        (01h)
  0Eh-0Fh                    (01EAh)    ;=245*2
  10h-11h                    (00F5h)    ;=245
  12h-13h                    (20h,F0h)  ;maybe min/max values?
  14h-15h                    (31h,C0h)  ;maybe min/max values?
  16h                        (00h)
  17h-18h                    (77h,77h)
  19h-1Ch                    (30h,30h,30h,30h)
  1Dh-20h                    (80h,80h,80h,80h)
  21h-FFh                    (not used by e-Reader BIOS)

This appears to be a Micron (aka Aptina) camera (resembling the DSi cameras).

My own e-Reader uses a Type 1 camera module. Not sure if Nintendo has ever
manufactured any e-Readers with Type 2 cameras?

Calibration Data in FLASH Memory (Bank 0, Sector 0Dh)

  E00D000 14h  ID String ('Card-E Reader 2001',0,0)
  E00D014 2    Sector Checksum (NOT(x+x/10000h); x=sum of all other halfwords)

Begin of actual data (40h bytes)

  E00D016 8x6  [00h] Intensity Boundaries for 8x6 blocks ;see E00FF80h..AFh
  E00D046 1    [30h] Vertical scroll (0..36h)  ;see type1.reg10h/type2.reg02h
  E00D047 1    [31h] Brightness or contrast    ;see type1.reg30h
  E00D048 2    [32h] LED Duration              ;see E00FFB2h..B3h
  E00D04A 2    [34h] Not used?   (0000h)
  E00D04C 2    [36h] Signed value, related to adjusting the 8x6 blocks
  E00D04E 4    [38h] Not used?   (00000077h)
  E00D052 4    [3Ch] Camera Type (0=none,1=DV488800,2=Whatever?)

Remaining bytes in this Sector...

  E00D056 FAAh Not used (zerofilled) (included in above checksum)

Flowchart for Overall Camera Access

ereader_scan_camera:

 call ereader_power_on
 call ereader_initialize
 for z=1 to number_of_frames
  for y=0 to 245
   Wait until E00FFB1h.Bit1 gets set by hardware (can be handled by IRQ)
   Copy 14h halfwords from DFC0000h to buf+y*28h via DMA3
   Reset E00FFB1h.Bit1 by software
  next y
  ;(could now check DFC0028h..DFC0086h/DFC0088h for adjusting E00FF00h..2Fh)
  ;(could now show image on screen, that may require to stop/pause scanning)
 next z
 call ereader_power_off
 Ret

ereader_power_on:

 [4000204h]=5803h   ;Init waitstates, and enable Phi 16MHz
 [DFA0000h].Bit1=1
 Wait(10ms)
 [E00FFB0h]=40h     ;Enable Power3V and reset other bits
 [DFA0000h].Bit1=0
 [E00FFB1h]=20h     ;Enable Power5V and reset other bits
 Wait(40ms)
 [E00FFB1h].Bit4=0  ;...should be already 0 ?
 [E00FFB0h]=40h+27h ;Phi16MHz=On, SioDtaClkDir=HighHighOut
 Ret

ereader_power_off:

 [E00FFB0h]=04h    ;Power3V=Off, Disable Everything, SioDtaClkDir=LowLowOut
 [DFA0000h].Bit1=0 ;...should be already 0
 [E00FFB1h].Bit5=0 ;Power5V=Off
 Ret

ereader_initialize:

 IF calib_data[3Ch] AND 03h = 1 THEN init_camera_type1
 [E00FFB0h].Bit4=1 ;ScanStart
 IF calib_data[3Ch] AND 03h = 2 THEN init_camera_type2
 Copy calib_data[00h..2Fh] to [E00FF80h+00h..2Fh]  ;Intensity Boundaries
 Copy calib_data[32h..33h] to [E00FFB2h+00h..01h]  ;LED Duration LSB,MSB
 [E00FFB0h].Bit3=1                                 ;LedIrqOn
 Ret

init_camera_type1:

 x=MIN(0,calib_data[31h]-0Bh)
 Set Sio Registers (as shown for Camera Type 1, except below values...)
 Set Sio Registers [30h]=x [25h]=04h, [26h]=58h, [27h]=6Ch
 ;(could now detect/adjust <x> based on Sio Registers [57h..5Ah])
 Set Sio Registers [30h]=x [25h]=06h, [26h]=E8h, [27h]=6Ch
 Ret

init_camera_type2:

 Wait(0.5ms)
 Set Sio Registers (as shown for Camera Type 2)
 Ret

Accessing Serial Registers via E00FFB0h

      Begin   Write(A) Write(B) Read(C) Read(D) End     Idle    PwrOff
  Dir ooooooo ooooooo  ooooooo  iiiiiii iiiiiii ooooooo ooooooo ooooooo
  Dta ---____ AAAAAAA  BBBBBBB  xxxxxCx xxxxxDx ______- ------- _______
  Clk ------_ ___---_  ___---_  ___---_ ___---_ ___---- ------- _______

Flowchart for accessing Serial Registers via E00FFB0h (looks like I2C bus)

 Delay:
  Wait circa 2.5us, Ret
 SioBegin:
  SioDta=1, SioDir=Out, SioClk=1, Delay, SioDta=0, Delay, SioClk=0, Ret
 SioEnd:
  SioDta=0, SioDir=Out, Delay, SioClk=1, Delay, SioDta=1, Ret
 SioRead1bit:   ;out: databit
  SioDir=In, Delay, SioClk=1, Delay, databit=SioDta, SioClk=0, Ret
 SioWrite1bit:  ;in: databit
  SioDta=databit, SioDir=Out, Delay, SioClk=1, Delay, SioClk=0, Ret
 SioReadByte:   ;in: endflag - out: data
  for i=7 to 0, data.bit<i>=SioRead1bit, next i, SioWrite1bit(endflag), Ret
 SioWriteByte:  ;in: data - out: errorflag
  for i=7 to 0, Delay(huh/why?), SioWrite1bit(data.bit<i>), next i
  errorflag=SioRead1bit, SioDir=Out(huh/why?), Ret
 SioWriteRegisters:  ;in: index, len, buffer
  SioBegin
  SioWriteByte(22h)        ;command (set_index) (and write_data)
  SioWriteByte(index)      ;index
  for i=0 to len-1
   SioWriteByte(buffer[i]) ;write data (and auto-increment index)
  next
  SioEnd
  ret
 SioReadRegisters:   ;in: index, len - out: buffer
  SioBegin
  SioWriteByte(22h)        ;command (set_index) (without any write_data here)
  SioWriteByte(index)      ;index
  SioBegin
  SioWriteByte(23h)        ;command (read_data) (using above index)
  for i=0 to len-1
   if i=len-1 then endflag=1 else endflag=0
   buffer[i]=SioReadByte(endflag)  ;read data (and auto-increment index)
  next
  SioEnd
  Ret

Caution: Accessing the SIO registers appears highly unstable, and seems to
require error handling with retries. Not sure what is causing that problem,
possibly the registers cannot be accessed during camera-data-scans...?

WAITCNT

The e-Reader BIOS uses WAITCNT [4000204h]=5803h when accessing the PGA, that
is, gamepak 16.78MHz phi output (bit11-12=3), 8 waits for SRAM region
(bit0-1=3), gamepak prefetch enabled (bit14=1), also sets WS0 to 4,2 waits
(bit2-4=0), and sets WS2 to odd 4,8 waits (bit8-10=0). The WS2 (probably WS0
too) settings are nonsense, and should work with faster timings (the e-Reader
can be accessed in NDS mode, which doesn't support that slow timings).

e-Reader Memory and I/O Map (with all used/unused/mirrored regions)

  C000000h-C7FFFFFh  ROM (8MB)
  C800000h-DF7FFFFh  Open Bus
  DF80000h-DF80001h  Useless Register (R/W)
  DF80002h-DF9FFFFh  Mirrors of DF80000h-DF80001h
  DFA0000h-DFA0001h  Reset Register (R/W)
  DFA0002h-DFBFFFFh  Mirrors of DFA0000h-DFA0001h
  DFC0000h-DFC0027h  Scanline Data (320 Pixels) (R)
  DFC0028h-DFC0087h  Brightest Pixels of 8x6 Blocks (R)
  DFC0088h           Darkest Pixel of whole Image (R)
  DFC0089h-DFC00FFh  Always zero
  DFC0100h-DFDFFFFh  Mirrors of DFC0000h-DFC00FFh
  DFE0000h-DFFFFFFh  Open Bus
  E000000h-E00CFFFh  FLASH Bank 0 - Data
  E00D000h-E00DFFFh  FLASH Bank 0 - Calibration Data
  E00E000h-E00EFFFh  FLASH Bank 0 - Copy of Calibration Data
  E00F000h-E00FF7Fh  FLASH Bank 0 - Unused region
  E000000h-E00EFFFh  FLASH Bank 1 - Data
  E00F000h-E00FF7Fh  FLASH Bank 1 - Unused region
  E00FF80h-E00FFAFh  Intensity Boundaries for 8x6 Blocks (R/W)
  E00FFB0h           Control Register 0 (R/W)
  E00FFB1h           Control Register 1 (R/W)
  E00FFB2h-E00FFB3h  LED Duration (16bit) (R/W)
  E00FFB4h-E00FFBFh  Always zero
  E00FFC0h-E00FFFFh  Mirror of E00FF80h-E00FFBFh

Mind that WS2 should be accessed by LDRH/STRH, and SRAM region by LDRB/STRB.

Additionally about 32 serial bus registers are contained in the camera module.

Camera Module Notes

The Type 1 initial setting on power-on is 402x302 pixels, the e-Reader uses
only 320x246 pixels. The full vertical resolution could be probably used
without problems. Port DFC0000h-DFC0027h are restricted to 320 pixels, so
larger horizontal resolutions could be probably obtained only by changing the
horizontal scroll offset on each 2nd scan.

The camera output is 128 grayscales (via parallel 7bit databus), but the PGA
converts it to 2 colors (1bit depth). For still images, it might be possible to
get 4 grayshades via 3 scans with different block intensity boundary settings.

No idea if the camera supports serial commands other than 22h and 23h. Namely,
it <would> be a quite obvious and basic feature to allow to receive the
bitmap via the 2-wire serial bus (alternately to the 7bit databus), if
supported, it'd allow to get 7bit images, bypassing 1bit PGA conversion.

When used as actual camera (by cutting an opening in the case), the main
problem is the 1bit color depth, which allows only black and white schemes,
when/if solving that problem, focusing might be also a problem.

Either the camera or the PGA seem to have a problem on white-to-black
transitions in vertical direction, the upper some black pixels are sorts of
getting striped or dithered. For example, scanning the large sync marks appears
as:

  Actual Shape    Scanned Shape
     XXXXX            X  X
    XXXXXXX           X  X X
   XXXXXXXXX        X X  X XX
   XXXXXXXXX        X X  X XX
    XXXXXXX          XXXXXXX
     XXXXX            XXXXX

That appears only on large black shapes (the smaller data dots look better).
Probably the image is scanned from bottom upwards (and the camera senses only
the initial transition at the bottom, and then looses track of what it is
doing).
```
