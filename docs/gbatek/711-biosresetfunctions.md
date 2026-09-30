# BIOS Reset Functions

Source: https://problemkaputt.de/gbatek.htm#biosresetfunctions

```
SoftReset

RegisterRamReset

HardReset

SWI 00h (GBA/NDS7/NDS9) - SoftReset

Clears 200h bytes of RAM (containing stacks, and BIOS IRQ vector/flags),
initializes system, supervisor, and irq stack pointers, sets R0-R12, LR_svc,
SPSR_svc, LR_irq, and SPSR_irq to zero, and enters system mode.

Note that the NDS9 stack registers are hardcoded (the DTCM base should be set
to the default setting of 0800000h). The NDS9 function additionally flushes
caches and write buffer, and sets the CP15 control register to 12078h.

  Host  sp_svc    sp_irq    sp_sys    zerofilled area       return address
  GBA   3007FE0h  3007FA0h  3007F00h  [3007E00h..3007FFFh]  Flag[3007FFAh]
  NDS7  380FFDCh  380FFB0h  380FF00h  [380FE00h..380FFFFh]  Addr[27FFE34h]
  NDS9  0803FC0h  0803FA0h  0803EC0h  [DTCM+3E00h..3FFFh]   Addr[27FFE24h]

The NDS7/NDS9 return addresses at [27FFE34h/27FFE24h] are usually containing
copies of Cartridge Header [034h/024h] entry points, which may select ARM/THUMB
state via bit0. The GBA return address 8bit flag is interpreted as 00h=8000000h
(ROM), or 01h-FFh=2000000h (RAM), entered in ARM state.

Note: The reset is applied only to the CPU that has executed the SWI (ie. on
the NDS, the other CPU will remain unaffected).

Return: Does not return to calling procedure, instead, loads the above return
address into R14, and then jumps to that address by a "BX R14" opcode.

SWI 01h (GBA) - RegisterRamReset

Resets the I/O registers and RAM specified in ResetFlags. However, it does not
clear the CPU internal RAM area from 3007E00h-3007FFFh.

  r0  ResetFlags
       Bit   Expl.
       0     Clear 256K on-board WRAM  ;-don't use when returning to WRAM
       1     Clear 32K on-chip WRAM    ;-excluding last 200h bytes
       2     Clear Palette
       3     Clear VRAM
       4     Clear OAM              ;-zerofilled! does NOT disable OBJs!
       5     Reset SIO registers    ;-switches to general purpose mode!
       6     Reset Sound registers
       7     Reset all other registers (except SIO, Sound)

Return: No return value.

Bug: LSBs of SIODATA32 are always destroyed, even if Bit5 of R0 was cleared.

The function always switches the screen into forced blank by setting
DISPCNT=0080h (regardless of incoming R0, screen becomes white).

SWI 26h (GBA) - HardReset (Undocumented)

This function reboots the GBA (including for getting through the time-consuming
nintendo intro, which is making the function particularly useless and
annoying).

Parameters: None. Return: Never/Reboot.

Execution Time: About 2 seconds (!)
```
