# ARM Opcodes: Special ARM11 Instructions (Misc)

Source: https://problemkaputt.de/gbatek.htm#armopcodesspecialarm11instructionsmisc

```
Load/Store Byte/Halfword/Doubleword Exclusive (LDREX/STREX)

  Bit    Expl.
  31-28  Condition
  27-23  Must be 00011b for this instruction
  22-20  Opcode
           0: STREX{cond}  Rd, Rm, [Rn]  ;[Rn]=Rm, Rd=flag       ;32bit
           1: LDREX{cond}  Rd, [Rn]      ;Rd=[Rm]                ;32bit
           2: STREXD{cond} Rd, Rm, [Rn]  ;[Rn]=Rm,Rm+1, Rd=flag  ;64bit
           3: LDREXD{cond} Rd, [Rn]      ;Rd,Rd+1=[Rm]           ;64bit
           4: STREXB{cond} Rd, Rm, [Rn]  ;[Rn]=Rm, Rd=flag       ;8bit
           5: LDREXB{cond} Rd, [Rn]      ;Rd=[Rm]                ;8bit
           6: STREXH{cond} Rd, Rm, [Rn]  ;[Rn]=Rm, Rd=flag       ;16bit
           7: LDREXH{cond} Rd, [Rn]      ;Rd=[Rm]                ;16bit
  19-16  Rn - Base Register                      (R0-R14)
  15-12  Rd - Destination Register               (R0-R14)
  11-4   Must be 1111.1001b for this instruction
  3-0    Rm - Source Register (1111b for LDREX)  (R0-R14)

For exclusive access to shared memory regions (ie. in dual-core CPUs?)

Unknown what "exclusive" means?

Return: No Flags affected.

ARMv6 and up supports LDREX/STREX (with 32bit operand).

ARMv6K and up supports LDREX/STREX{B|H|D} (with 8/16/64bit operand).

Clear Exclusive (CLREX)

  Bit    Expl.
  31-0   Opcode 1111.0101.0111.1111.1111.0000.0001.1111b for CLREX

Supported on ARMv6K and up.

Hint (TrueNOP, YIELD, WFE, WFI, SEV)

  Bit    Expl.
  31-28  Condition (should be always 0Eh=Always for NOP)
  27-8   Must be 0011.0010.0000.1111.0000b
  7-0    Opcode/Hint
           00h: TrueNOP      ;no operation
           01h: YIELD{cond}
           02h: WFE{cond}    ;wait for event
           03h: WFI{cond}    ;wait for interrupt
           04h: SEV{cond}    ;send event

Supported on ARMv6K and up.

Change Processor State aka CPSR Mode/Flags (CPS, CPSID, CPSIE)

  31-20  Must be 1111.0001.0000b
  19-17  Opcode
          001b: CPS   #<mode>           ;change mode
          100b: CPSIE {A}{I}{F}         ;interrupt bit(s) enable
          101b: CPSIE {A}{I}{F},#<mode> ;interrupt bit(s) enable & change mode
          110b: CPSID {A}{I}{F}         ;interrupt bit(s) disable
          111b: CPSID {A}{I}{F},#<mode> ;interrupt bit(s) disable & change mode
  16-9   Must be 0000.0000b
  8      Change A flag (what?) (0=No, 1=Yes)  ;\must be all zero for CPS,
  7      Change I flag         (0=No, 1=Yes)  ; must be at least one flag set
  6      Change F flag         (0=No, 1=Yes)  ;/for CPSID/CPSIE
  5      Must be 0
  4-0    Mode (00h..1Fh) (must be 00h if bit17=0)

Set Endianness for Data Access (SETEND)

  31-10  Fixed, 1111000100000001000000b
  9      Opcode/operand
          0: SETEND LE  ;clears the CPSR E bit
          1: SETEND BE  ;sets the CPSR E bit
  8-0    Fixed, 000000000b

Changes the byte order for data access to Big/Little endian. Does probably not
affect opcode fetches, but probably affect all other memory including push/pop?
Or affects "LDR" only?

Supported on ARMv6 and above.

Store Return State (SRS) (stores the R14 and SPSR of the current mode)

to the word at the specified address and the following word respectively. The
address is determined from the banked version of R13 belonging to a specified
mode.

  Syntax
  SRS<addressing_mode> #<mode>{!}   ;<-- original (pre-UAL)
  SRS<addressing_mode> sp!,#<mode>  ;<-- more common (UAL)

<addressing_mode>: Is similar to the <addressing_mode> in LDM and
STM instructions, see Addressing Mode 4 - Load and Store Multiple on page
A5-41, but with the following differences:

- The base register, Rn, is the banked version of R13 for the mode specified by
<mode>, rather than the current mode.

- The number of registers to store is 2.

- The register list is {R14, SPSR}, with both R14 and the SPSR being the
versions belonging to the current mode.

<mode> Specifies the number of the mode whose banked register is used as
the base register for <addressing_mode>. The mode number is the 5-bit
encoding of the chosen mode in a PSR, as described in The mode bits on page
A2-14.

! If present, sets the W bit. This causes the instruction to write a modified
value back to its base register, in a manner similar to that specified for
Addressing Mode 4 - Load and Store Multiple on page A5-41. If ! is omitted, the
W bit is 0 and the instruction does not change the base register.

  31-25  Fixed, 1111100b
  24     P
  23     U
  22     Fixed, 1
  21     W
  20     Fixed, 0 (store)
  19-5   Fixed, 110100000101000b
  4-0    mode

Supported on ARMv6 and above.

Return From Exception (RFE) (RFEDA, RFEDB, RFEIA, RFEIB)

loads the PC and the CPSR from the word at the specified address and the

following word respectively.

  Syntax
  RFE<addressing_mode> Rn{!}

<addressing_mode>: Is similar to the <addressing_mode> in LDM and
STM instructions, see Addressing Mode 4 - Load and Store Multiple on page
A5-41, but with the following differences:

- The number of registers to load is 2.

- The register list is {PC, CPSR}.

<Rn> Specifies the base register to be used by <addressing_mode>.
If R15 is specified as the base register, the result is UNPREDICTABLE.

! If present, sets the W bit. This causes the instruction to write a modified
value back to its base register, in a manner similar to that specified for
Addressing Mode 4 - Load and Store Multiple on page A5-41. If ! is omitted, the
W bit is 0 and the instruction does not change the base register.

While RFE supports different base registers, a general usage case is where Rn
== sp (the stack pointer), held in R13. The instruction can then be used as the
return method associated with instructions SRS and CPS. See New instructions to
improve exception handling on page A2-28 for more details.

  31-25  Fixed, 1111100b
  24     P
  23     U
  22     Fixed, 0
  21     W
  20     Fixed, 1 (load)
  19-16  Rn
  15-0   Fixed, 0000101000000000b

Supported on ARMv6 and above.

SVC (formerly SWI)

SWI was renamed to SVC in new UAL syntax.

CPY

CPY Rd,Rm is an alias for MOV Rd,Rm (without flags affected).

  31-28  Condition
  27-16  Fixed, 000110100000
  15-12  Rd - Destination Register
  11-4   Fixed, 00000000
  3-0    Rm

The alias was invented in ARMv6 for THUMB and ARM code (apparently because
pre-UAL THUMB syntax couldn't distinguish between MOV and MOVS) (ie. in ARM
it's useless, but in THUMB it does allow using a formerly undocumented opcode).

ARMv6 and above.
```
