# ARM CP15 Tightly Coupled Memory (TCM)

Source: https://problemkaputt.de/gbatek.htm#armcp15tightlycoupledmemorytcm

```
TCM is high-speed memory, directly contained in the ARM CPU core.

TCM and DMA

TCM doesn't use the ARM bus. A minor disadvantage is that TCM cannot be
accessed by DMA. However, the main advantage is that, when using TCM, the CPU
can be kept running without any waitstates even while the bus is used for DMA
transfers. Operation during DMA works only if all code/data is located in TCM,
waitstates are generated if any code/data outside TCM is accessed; in worst
case (if there are no gaps in the DMA) then the CPU is halted until the DMA
finishes.

TCM and DMA and IRQ

No idea if/how IRQs are handled during DMA? Eventually (unlikely) code in TCM
is kept executed until DMA finishes (ie. until the IRQ vector can be accessed.
Eventually the IRQ vector is instantly accessed (causing to halt the CPU until
DMA finishes). In both cases: Assuming that IRQs are enabled, and that the IRQ
vector and/or IRQ handler are located outside TCM.

Separate Instruction (ITCM) and Data (DTCM) Memory

DTCM can be used only for Data accesses, typically used for stacks and other
frequently accessed data.

ITCM is primarily intended for instruction accesses, but it can be also used
for Data accesses (among others allowing to copy code to ITCM), however,
performance isn't optimal when simultaneously accessing ITCM for code and data
(such like opcodes in ITCM that use literal pool values in ITCM).

TCM Enable, TCM Load Mode

CP15 Control Register allows to enable ITCM and DTCM, and to switch ITCM/DTCM
into Load Mode. In Load Mode (when TCM is enabled), TCM becomes write-only;
this allows to read data from source addresses in main memory, and to write
data to destination addresses in TCM by using the same addresses; useful for
initializing TCM with overlapping source/dest addresses; Load mode works with
all Load/Store opcodes, it does NOT work with SWP/SWPB opcodes.

TCM Physical Size can be detected in 3rd ID Code Register. (C0,C0,2)

C9,C1,0 - Data TCM Size/Base (R/W)

C9,C1,1 - Instruction TCM Size/Base (R/W)

  0     Reserved     (0)
  1-5   Virtual Size (Size = 512 SHL N) ;min=(N=3)=4KB, max=(N=23)=4GB
  6-11  Reserved     (0)
  12-31 Region Base  (Base = X SHL 12)  ;Base must be Size-aligned

The Virtual size settings should be normally same as the Physical sizes (see
C0,C0,2). However, smaller sizes are allowed (using only the 1st some KB), as
well as bigger sizes (TCM area is then filled with mirrors of physical TCM).

The ITCM region base may be fixed (read-only), for example, on the NDS, ITCM
base is always 00000000h, nethertheless the virtual size may be changed
(allowing to mirror ITCM to higher addresses).

If DTCM and ITCM do overlap, then ITCM appears to have priority.

TCM and PU

TCM can be used without Protection Unit.

When the protection unit is enabled, TCM is controlled by the PU just like
normal memory, the PU should provide R/W Access Permission for TCM regions;
cache and write-buffer are not required for high-speed TCM (so both should be
disabled for TCM regions).
```
