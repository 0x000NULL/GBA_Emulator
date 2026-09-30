# ARM CP15 Overview

Source: https://problemkaputt.de/gbatek.htm#armcp15overview

```
CP15

In many ARM CPUs, particulary such with memory control facilities, coprocessor
number 15 (CP15) is used as built-in System Control Coprocessor.

CPUs without memory control functions typically don't include a CP15 at all, in
that case even an attempt to read the Main ID register will cause an Undefined
Instruction exception.

CP15 Opcodes

CP15 can be accessed via MCR and MRC opcodes, with Pn=P15, and <cpopc>=0.

  MCR{cond} P15,<cpopc>,Rd,Cn,Cm,<cp>   ;move from ARM to CP15
  MRC{cond} P15,<cpopc>,Rd,Cn,Cm,<cp>   ;move from CP15 to ARM

Rd can be any ARM register in range R0-R14, R15 should not be used with P15.

<cpopc>,Cn,Cm,<cp> are used to select a CP15 register, eg.
0,C0,C0,0 = Main ID Register.

Other coprocessor opcodes (CDP, LDC, STC) cannot be used with P15.

CP15 Register List

  Register       Expl.
  0,C0,C0,0      Main ID Register (R)
  0,C0,C0,1      Cache Type and Size (R)
  0,C0,C0,2      TCM Physical Size (R)
  0,C0,C0,3      ARM11: TLB Type Register
  0,C0,C1,0      ARM11: Processor Feature Register 0
  0,C0,C1,1      ARM11: Processor Feature Register 1
  0,C0,C1,2      ARM11: Debug Feature Register 0
  0,C0,C1,3      ARM11: Auxiliary Feature Register 0
  0,C0,C1,4      ARM11: Memory Model Feature Register 0
  0,C0,C1,5      ARM11: Memory Model Feature Register 1
  0,C0,C1,6      ARM11: Memory Model Feature Register 2
  0,C0,C1,7      ARM11: Memory Model Feature Register 3
  0,C0,C2,0      ARM11: Set Attributes Register 0
  0,C0,C2,1      ARM11: Set Attributes Register 1
  0,C0,C2,2      ARM11: Set Attributes Register 2
  0,C0,C2,3      ARM11: Set Attributes Register 3
  0,C0,C2,4      ARM11: Set Attributes Register 4
  0,C0,C2,5      ARM11: Set Attributes Register 5

  0,C1,C0,0      Control Register (R/W, or R=Fixed)
  0,C1,C0,1      ARM11: Auxiliary Control Register
  0,C1,C0,2      ARM11: Coprocessor Access Control Register
  0,C2,C0,0      ARM11: Translation Table Base Register 0
  0,C2,C0,1      ARM11: Translation Table Base Register 1
  0,C2,C0,2      ARM11: Translation Table Base Control Register
  0,C3,C0,0      ARM11: Domain Access Control Register
  0,C5,C0,0      ARM11: Data Fault Status Register
  0,C5,C0,1      ARM11: Instruction Fault Status Register
  0,C6,C0,0      ARM11: Fault Address Register (FAR)
  0,C6,C0,1      ARM11: Watchpoint Fault Address Register (WFAR)
  0,C2,C0,0      PU Cachability Bits for Data/Unified Protection Region
  0,C2,C0,1      PU Cachability Bits for Instruction Protection Region
  0,C3,C0,0      PU Cache Write-Bufferability Bits for Data Protection Regions
  0,C5,C0,0      PU Access Permission Data/Unified Protection Region
  0,C5,C0,1      PU Access Permission Instruction Protection Region
  0,C5,C0,2      PU Extended Access Permission Data/Unified Protection Region
  0,C5,C0,3      PU Extended Access Permission Instruction Protection Region
  0,C6,C0..C7,0  PU Protection Unit Data/Unified Region 0..7
  0,C6,C0..C7,1  PU Protection Unit Instruction Region 0..7
  0,C7,Cm,Op2    Cache Commands and Halt Function (W)
  0,C9,C0,0      Cache Data Lockdown
  0,C9,C0,1      Cache Instruction Lockdown
  0,C9,C1,0      TCM Data TCM Base and Virtual Size
  0,C9,C1,1      TCM Instruction TCM Base and Virtual Size
  0,C13,Cm,Op2   Misc Process ID registers
  0,C15,Cm,Op2   Misc Implementation Defined and Test/Debug registers

Data/Unified Registers

Some Cache/PU/TCM registers are declared as "Data/Unified".

That registers are used for Data accesses in case that the CPU contains
separate Data and Instruction registers, otherwise the registers are used for
both (unified) Data and Instruction accesses.
```
