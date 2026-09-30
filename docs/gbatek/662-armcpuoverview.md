# ARM CPU Overview

Source: https://problemkaputt.de/gbatek.htm#armcpuoverview

```
The ARM7TDMI is a 32bit RISC (Reduced Instruction Set Computer) CPU, designed
by ARM (Advanced RISC Machines), and designed for both high performance and low
power consumption.

Fast Execution

Depending on the CPU state, all opcodes are sized 32bit or 16bit (that's
counting both the opcode bits and its parameters bits) providing fast decoding
and execution. Additionally, pipelining allows - (a) one instruction to be
executed while (b) the next instruction is decoded and (c) the next instruction
is fetched from memory - all at the same time.

Data Formats

The CPU manages to deal with 8bit, 16bit, and 32bit data, that are called:

   8bit - Byte
  16bit - Halfword
  32bit - Word

The two CPU states

As mentioned above, two CPU states exist:

- ARM state: Uses the full 32bit instruction set (32bit opcodes)

- THUMB state: Uses a cutdown 16bit instruction set (16bit opcodes)

Regardless of the opcode-width, both states are using 32bit registers, allowing
32bit memory addressing as well as 32bit arithmetic/logical operations.

When to use ARM state

Basically, there are two advantages in ARM state:

 - Each single opcode provides more functionality, resulting
   in faster execution when using a 32bit bus memory system
   (such like opcodes stored in GBA Work RAM).
 - All registers R0-R15 can be accessed directly.

The downsides are:

 - Not so fast when using 16bit memory system
   (but it still works though).
 - Program code occupies more memory space.

When to use THUMB state

There are two major advantages in THUMB state:

 - Faster execution up to approx 160% when using a 16bit bus
   memory system (such like opcodes stored in GBA GamePak ROM).
 - Reduces code size, decreases memory overload down to approx 65%.

The disadvantages are:

 - Not as multi-functional opcodes as in ARM state, so it will
   be sometimes required use more than one opcode to gain a
   similar result as for a single opcode in ARM state.
 - Most opcodes allow only registers R0-R7 to be used directly.

Combining ARM and THUMB state

Switching between ARM and THUMB state is done by a normal branch (BX)
instruction which takes only a handful of cycles to execute (allowing to change
states as often as desired - with almost no overload).

Also, as both ARM and THUMB are using the same register set, it is possible to
pass data between ARM and THUMB mode very easily.

The best memory & execution performance can be gained by combining both
states: THUMB for normal program code, and ARM code for timing critical
subroutines (such like interrupt handlers, or complicated algorithms).

Note: ARM and THUMB code cannot be executed simultaneously.

Automatic state changes

Beside for the above manual state switching by using BX instructions, the
following situations involve automatic state changes:

- CPU switches to ARM state when executing an exception

- User switches back to old state when leaving an exception
```
