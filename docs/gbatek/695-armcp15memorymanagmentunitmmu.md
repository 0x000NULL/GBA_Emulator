# ARM CP15 Memory Managment Unit (MMU)

Source: https://problemkaputt.de/gbatek.htm#armcp15memorymanagmentunitmmu

```
Nintendo consoles have a MMU on the ARM11 processor (in 3DS).

The MMU allows to assign virtual memory addresses, and to disable the data
cache in sensitive memory areas (eg. I/O area, and memory that is shared for
ARM9/ARM11). Additionally, it can trigger page fault exceptions (eg. for error
handling or memory mapped files).

Related CP15 registers used in 3DS bootrom MMU init

  p15,0,c8,c5,0 ;Invalidate Instruction TLB               (=0)
  p15,0,c8,c6,0 ;Invalidate Data TLB                      (=0)
  p15,0,c2,c0,0 ;Translation Table Base 0 (for process)   (=1FFF4000h+2)
  p15,0,c2,c0,1 ;Translation Table Base 1 (for OS and I/O)(=1FFF4000h+2)
  p15,0,c2,c0,2 ;Translation Table Base Control           (=0)
  p15,0,c3,c0,0 ;Domain Access Control                    (=55555555h)
  p15,0,c1,c0,2 ;Coprocessor Access Control (unrelated?)  (=0F00000h)

MMU Tables

1st Level table (size 4000h) divides 4Gbyte address space into 1Mbyte sections

2nd Level table(s) (size 400h each) divides a 1Mbyte section into 4Kbyte pages

 ________________________________ ARMv6 Tables ________________________________

ARMv6 First-Level Page Table entries

   3 3 2 2 2 2 2 2 2 2 2 2 1 1 1 1 1 1 1 1 1 1
   1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0
  |__Ignored__________________________________________________|0_0| Fault
  |__Second-Level-Table-Address_______________|P|Domain_|_SBZ_|0_1| Coarse
  |__Address______________|0|0|n_S_A|_TEX_|AP_|P|Domain_|X_C_B|1_0| Section 1MB
  |__Address______|___SBZ___|1|_SBZ_|_TEX_|AP_|P|Ignored|X_C_B|1_0| Super 16MB
  |___________________________________________________________|1_1| Reserved

BUG: The Supersections do probably also have "n,S,A" flags, probably in
bit17-15, too (the 3DS bootrom seems to set those bits as so).

ARMv6 Second-Level Page Table entries

   3 3 2 2 2 2 2 2 2 2 2 2 1 1 1 1 1 1 1 1 1 1
   1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0
  |__Ignored__________________________________________________|0_0| Fault
  |__Address______________________|X|_TEX_|n_S_A|_SBZ_|AP_|C_B|0_1| Large 64K
  |__Address______________________________|n_S_A|_TEX_|AP_|C_B|1_X| Small 4K

 ______________________________ Pre-ARMv6 Tables ______________________________

This is an older format, similar to the newer ARMv6 tables, but without the
"n,S,A,X" bits, and, instead, somehow allowing to divide 4K pages into smaller
1K snippets (with separate access privileges in AP0/AP1/AP2/AP3 bits, but
without TEX bits).

Backwards-compatible First-Level Page Table entries

   3 3 2 2 2 2 2 2 2 2 2 2 1 1 1 1 1 1 1 1 1 1
   1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0
  |__Ignored__________________________________________________|0_0| Fault
  |__Second-Level-Table-Address_______________|P|Domain_|_SBZ_|0_1| Coarse
  |__Address______________|0|0|_SBZ_|_TEX_|AP_|P|Domain_|0_C_B|1_0| Section 1MB
  |__Address______|___SBZ___|1|_SBZ_|_TEX_|AP_|P|Ignored|0_C_B|1_0| Super 16MB
  |___________________________________________________________|1_1| Reserved

Backwards-compatible Second-Level Page Table entries

   3 3 2 2 2 2 2 2 2 2 2 2 1 1 1 1 1 1 1 1 1 1
   1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0
  |__Ignored__________________________________________________|0_0| Fault
  |__Address______________________|_?TEX?_|AP3|AP2|AP1|AP0|C_B|0_1| Large 4x16K
  |__Address______________________________|AP3|AP2|AP1|AP0|C_B|1_0| Small 4x1K
  |__Address______________________________|_SBZ_|_TEX_|AP_|C_B|1_1| Extended 4K

BUG: The 4bit ?TEX? value is probably meant to be a 3bit TEX value, probably
with an unused/zero bit; in bit15 or so.

 ___________________________________ Notes ___________________________________

Supersections and Large Pages

16Mbyte Supersections and 64Kbyte Large pages consist of 16 identical table
entries. Which is basically same as 16 Sections or 16 Pages with continously
increasing entries.

The advantage is internal caching: If one table entry is cached, then the
hardware knows that the other 15 nearby entries are supposed to be same; and
that they won't need to be loaded into internal cache.
```
