# BIOS Halt Functions

Source: https://problemkaputt.de/gbatek.htm#bioshaltfunctions

```
Halt

IntrWait

VBlankIntrWait

Stop/Sleep

CustomHalt

SWI 02h (GBA) or SWI 06h (NDS7/NDS9/DSi7/DSi9) - Halt

Halts the CPU until an interrupt request occurs. The CPU is switched into
low-power mode, all other circuits (video, sound, timers, serial, keypad,
system clock) are kept operating.

Halt mode is terminated when any enabled interrupts are requested, that is when
(IE AND IF) is not zero, the GBA locks up if that condition doesn't get true.
However, the state of CPUs IRQ disable bit in CPSR register, and the IME
register are don't care, Halt passes through even if either one has disabled
interrupts.

On GBA and NDS7/DSi7, Halt is implemented by writing to HALTCNT, Port 4000301h.
On NDS9/DSi9, Halt is implemted by writing to System Control Coprocessor (mov
p15,0,c7,c0,4,r0 opcode), this opcode hangs if IME=0.

No parameters, no return value.

(GBA/NDS7/DSi7: all registers unchanged, NDS9/DSi9: R0 destroyed)

SWI 04h (GBA/NDS7/NDS9/DSi7/DSi9) - IntrWait ;DSi7/DSi9=bugged?

Continues to wait in Halt state until one (or more) of the specified
interrupt(s) do occur. The function forcefully sets IME=1. When using multiple
interrupts at the same time, this function is having less overhead than
repeatedly calling the Halt function.

  r0    0=Return immediately if an old flag was already set (NDS9: bugged!)
        1=Discard old flags, wait until a NEW flag becomes set
  r1    Interrupt flag(s) to wait for (same format as IE/IF registers)
  r2    DSi7 only: Extra flags (same format as DSi7's IE2/IF2 registers)

Caution: When using IntrWait or VBlankIntrWait, the user interrupt handler MUST
update the BIOS Interrupt Flags value in RAM; when acknowleding processed
interrupt(s) by writing a value to the IF register, the same value should be
also ORed to the BIOS Interrupt Flags value, at following memory location:

  Host     GBA (16bit)  NDS7 (32bit)  NDS9 (32bit)  DSi7-IF2 (32bit)
  Address  [3007FF8h]   [380FFF8h]    [DTCM+3FF8h]  [380FFC0h]

NDS9: BUG: No Discard (r0=0) doesn't work. The function always waits for at
least one IRQ to occur (no matter which, including IRQs that are not selected
in r1), even if the desired flag was already set. NB. the same bug is also
found in the GBA/NDS7 functions, but it's compensated by a second bug, ie. the
GBA/NDS7 functions are working okay because their "bug doesn't work".

Return: No return value, the selected flag(s) are automatically reset in BIOS
Interrupt Flags value in RAM upon return.

DSi9: BUG: The function tries to enter Halt state via Port 4000301h (which
would be okay on ARM7, but it's probably ignored on ARM9, which should normally
use CP15 to enter Halt state; if Port 4000301h is really ignored, then the
function will "successfully" wait for interrupts, but without actually entering
any kind of low power mode).

DSi7: BUG: The function tries to wait for IF and IF2 interrupts, but it does
accidently ignore the old IF interrupts, and works only with new IF2 ones.

SWI 05h (GBA/NDS7/NDS9/DSi7/DSi9) - VBlankIntrWait ;DSi7/DSi9=bugged?

Continues to wait in Halt status until a new V-Blank interrupt occurs.

The function sets r0=1 and r1=1 (plus r2=0 on DSi7) and does then execute
IntrWait (SWI 04h), see IntrWait for details.

No parameters, no return value.

SWI 03h (GBA) - Stop

Switches the GBA into very low power mode (to be used similar as a
screen-saver). The CPU, System Clock, Sound, Video, SIO-Shift Clock, DMAs, and
Timers are stopped.

Stop state can be terminated by the following interrupts only (as far as
enabled in IE register): Joypad, Game Pak, or General-Purpose-SIO.

"The system clock is stopped so the IF flag is not set."

Preparation for Stop:

Disable Video before implementing Stop (otherwise Video just freezes, but still
keeps consuming battery power). Possibly required to disable Sound also?
Obviously, it'd be also recommended to disable any external hardware (such like
Rumble or Infra-Red) as far as possible.

No parameters, no return value.

SWI 07h (NDS7/DSi7) - Sleep

No info, probably similar as GBA SWI 03h (Stop). Sleep is implemented for ARM7
only, not for ARM9. But maybe the ARM7 function does stop <both> ARM7 and
ARM9 (?)

SWI 27h (GBA) or SWI 1Fh (NDS7/DSi7) - CustomHalt (Undocumented)

Writes the 8bit parameter value to HALTCNT, below values are equivalent to Halt
and Stop/Sleep functions, other values reserved, purpose unknown.

  r2  8bit parameter (GBA: 00h=Halt, 80h=Stop) (NDS7/DSi7: 80h=Halt, C0h=Sleep)

No return value.
```
