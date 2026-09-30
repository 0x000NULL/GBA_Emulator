# BIOS Functions

Source: https://problemkaputt.de/gbatek.htm#biosfunctions

```
The BIOS includes several System Call Functions which can be accessed by SWI
instructions. Incoming parameters are usually passed through registers
R0,R1,R2,R3. Outgoing registers R0,R1,R3 are typically containing either
garbage, or return value(s). All other registers (R2,R4-R14) are kept
unchanged.

Caution

When invoking SWIs from inside of ARM state specify SWI NN*10000h, instead of
SWI NN as in THUMB state.

Overview

BIOS Function Summary

BIOS Differences between GBA and NDS functions

All Functions Described

BIOS Arithmetic Functions

BIOS Rotation/Scaling Functions

BIOS Decompression Functions

BIOS Memory Copy

BIOS Halt Functions

BIOS Reset Functions

BIOS Misc Functions

BIOS Multi Boot (Single Game Pak)

BIOS Sound Functions

BIOS SHA1 Functions (DSi only)

BIOS RSA Functions (DSi only)

3DS bootroms

BIOS 3DS Exception Vectors

RAM Usage, BIOS Dumps

BIOS RAM Usage

BIOS Dumping

How BIOS Processes SWIs

SWIs can be called from both within THUMB and ARM mode. In ARM mode, only the
upper 8bit of the 24bit comment field are interpreted.

Each time when calling a BIOS function 4 words (SPSR, R11, R12, R14) are saved
on Supervisor stack (_svc). Once it has saved that data, the SWI handler
switches into System mode, so that all further stack operations are using user
stack.

In some cases the BIOS may allow interrupts to be executed from inside of the
SWI procedure. If so, and if the interrupt handler calls further SWIs, then
care should be taken that the Supervisor Stack does not overflow.
```
