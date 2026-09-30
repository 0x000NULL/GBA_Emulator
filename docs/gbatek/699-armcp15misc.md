# ARM CP15 Misc

Source: https://problemkaputt.de/gbatek.htm#armcp15misc

```
C13,C0,0 - Process ID for Fast Context Switch Extension (FCSE) (R/W)

  0-24  Reserved/zero
  25-31 Process ID (PID) (0-127) (0=Disable)

The FCSE allows different processes (each assembled with ORG 0) to be located
at virtual addresses in the 1st 32MB area. The FCSE splits the total 4GB
address space into blocks of 32MB, accesses to Block(0) are redirected to
Block(PID):

  IF addr<32M then addr=addr+PID*32M
  Respectively, with PID=0, the address remains unchanged (FCSE disabled).

The CPU-to-Memory address handling is shown below:

  1. CPU outputs a virtual address (VA)
  2. FCSE adjusts the VA to a modified virtual address (MVA)
  3. Cache hits determined by examining the MVA, continue below if no hit
  4. MMU translates MVA to physical address (PA) (if no MMU present: PA=MVA)
  5. Memory access occurs at PA

The FCSE allows limited virtual addressing even if no MMU is present.

If the MMU is present, then either the FCSE and/or the MMU can be used for
virtual addressing; the advantage of using the FCSE (a single write to
C13,C0,0) is less overload; using the MMU for the same purpose would require to
change virtual address translation table in memory, and to flush the cache.

The NDS doesn't have a FCSE (the FCSE register is read-only, always zero).

C13,C0,1 - Trace Process ID (R/W)

C13,C1,1 - Trace Process ID (Mirror) (R/W)

This value is output to ETMPROCID pins (if any), allowing to notify external
hardware about the currently executed process within multi-tasking programs.

  0-31  Process ID

C13,C1,1 is a mirror of C13,C0,1 (for compatibility with other ARM processors).

Both registers are read/write-able on NDS9, but there are no external pin-outs.

<cpopc>

Unlike for all other CP15 registers, the <cpopc> operand of the MRC/MCR
opcodes isn't always zero for below registers, so below registers are using
"cpopc,Cn,Cm,op2" notation (instead of the normal "Cn,Cm,op2" notation).

Built-In-Self-Test (BIST)

Allows to test internal memory (ie. TCM, Cache Memory, and Cache TAGs). The
tests are filling (and verifying) the selected memory region thrice (once with
the fillvalue, then with the inverted fillvalue, and then again with the
fillvalue). The BIST functions are intended for diagnostics purposes only, not
for use in normal program code (ARM doesn't guarantee future processors to have
backwards compatible BIST functions).

0,C15,C0,1 - BIST TAG Control Register (R/W)

1,C15,C0,1 - BIST TCM Control Register (R/W)

2,C15,C0,1 - BIST Cache Control Register (R/W)

  0-15  Data Control (see below)
  16-31 Instruction Control (see below)

The above 16bit control values are:

  0     Start bit     (Write: 1=Start) (Read: 1=Busy)
  1     Pause bit     (1=Pause)
  2     Enable bit    (1=Enable)
  3     Fail Flag     (1=Error) (Read Only)
  4     Complete Flag (1=Ready) (Read Only)
  5-15  Size (2^(N+2) bytes) (min=N=1=8bytes, max=N=24=64MB)

Size and Pause are not supported in all implementations.

Caution: While and as long as the Enable bit is set, the corresponding memory
region(s) will be disabled. Eg. when testing <either> DTCM <and/or>
ITCM, <both> DTCM <and> ITCM are forcefully disabled in C1,C0,0
(Control Register), after the test the software must first clear the BIST
enable bit, and then restore DTCM/ITCM bits in C1,C0,0. And of course, the
content of the tested memory region must be restored when needed.

0,C15,C0,2 - BIST Instruction TAG Address (R/W)

1,C15,C0,2 - BIST Instruction TCM Address (R/W)

2,C15,C0,2 - BIST Instruction Cache Address (R/W)

0,C15,C0,6 - BIST Data TAG Address (R/W)

1,C15,C0,6 - BIST Data TCM Address (R/W)

2,C15,C0,6 - BIST Data Cache Address (R/W)

  0-31  Word-aligned Destination Address within Memory Block (eg. within ITCM)

On the NDS9, bit0-1, and bit21-31 are always zero.

0,C15,C0,3 - BIST Instruction TAG Fillvalue (R/W)

1,C15,C0,3 - BIST Instruction TCM Fillvalue (R/W)

2,C15,C0,3 - BIST Instruction Cache Fillvalue (R/W)

0,C15,C0,7 - BIST Data TAG Fillvalue (R/W)

1,C15,C0,7 - BIST Data TCM Fillvalue (R/W)

2,C15,C0,7 - BIST Data Cache Fillvalue (R/W)

  0-31  Fillvalue for BIST

After BIST, the selected memory region is filled by that value. That is, on the
NDS9 at least, all words will be filled with the SAME value (ie. NOT with
increasing or randomly generated numbers).

0,C15,C0,0 - Cache Debug Test State Register (R/W)

  0-8    Reserved (zero)
  9      Disable Instruction Cache Linefill
  10     Disable Data Cache Linefill
  11     Disable Instruction Cache Streaming
  12     Disable Data Cache Streaming
  13-31  Reserved (zero/unpredictable)

3,C15,C0,0 - Cache Debug Index Register (R/W)

  0..1    Reserved (zero)
  2..4    Word Address
  5..N    Index
  N+1..29 Reserved (zero)
  30..31  Segment

3,C15,C1,0 - Cache Debug Instruction TAG (R/W)

3,C15,C2,0 - Cache Debug Data TAG (R/W)

3,C15,C3,0 - Cache Debug Instruction Cache (R/W)

3,C15,C4,0 - Cache Debug Data Cache (R/W)

  0..1    Set
  2..3    Dirty Bits
  4       Valid
  5..N    Index
  N+1..31 TAG Address
```
