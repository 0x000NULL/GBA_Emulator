# THUMB Opcodes: New THUMB Opcodes in ARM11

Source: https://problemkaputt.de/gbatek.htm#thumbopcodesnewthumbopcodesinarm11

```
THUMB.X: signed/unsigned extend byte/halfword (ARMv6)

These unneccessary opcodes are apparently meant to reduce typical compiler
overload (eg. for ensuring that "for i=0 to 7" won't end up with i>255).

  15-8   Must be 1011.0010b for this type of instructions
  7-6    Opcode
          00b  SXTH  Rd,Rm      ;sign-extend halfword
          01b  SXTB  Rd,Rm      ;sign-extend byte
          10b  UXTH  Rd,Rm      ;zero-extend halfword
          11b  UXTB  Rd,Rm      ;zero-extend byte
  5-3    Rm Source Register                    (R0-R7)
  2-0    Rd Destination Register               (R0-R7)

Return: No flags affected (though unspecified?).

Execution Time: ?

THUMB.X: byte-reverse word/halfword(s) (ARMv6)

  15-8   Must be 1011.1010b for this type of instructions
  7-6    Opcode
          00b  REV   Rd,Rm      ;swap byte3/byte0 and byte2/byte1  ;word
          01b  REV16 Rd,Rm      ;swap byte3/byte2 and byte1/byte0  ;packed half
          11b  REVSH Rd,Rm      ;swap byte1/byte0 and sign-extend  ;signed half
  5-3    Rm Source Register                    (R0-R7)
  2-0    Rd Destination Register               (R0-R7)

Return: No flags affected (though unspecified?).

Execution Time: ?

SETEND (ARMv6)

  15-4   Must be 101101100101b
  3      Opcode:
           0: SETEND LE   ;clear CPSR.E bit (little endian data access)
           1: SETEND BE   ;set CPSR.E bit (big endian data access)
  2-0    Should be 000b

THUMB.X: change processor state (ARMv6)

  15-5   Must be 10110110011b
  4      Opcode:
           0: CPSIE {A}{I}{F}   ;Interrupt Enable
           1: CPSID {A}{I}{F}   ;Interrupt Disable
  3      Must be 0b
  2      A  Affect CPSR.A (?)    (0=No, 1=Yes)
  1      I  Affect CPSR.I (IRQ)  (0=No, 1=Yes)
  0      F  Affect CPSR.F (FIQ)  (0=No, 1=Yes)

CPY

  15-8   Must be 01000110b for CPY Rd,Rm
  7      MSB of Rd
  6      MSB of Rm
  5-3    Rm Source Register
  2-0    Rd Destination Register

Move without changing flags. It's called CPY because THUMB syntax didn't
distinguish between MOV and MOVS (until UAL syntax was invented, which makes
the name CPY irrelevant).

CPY is officially supported in "T variants of ARMv6 and above" (in practice, it
should work on ARMv4T and up, but wasn't officially documented until ARMv6).
```
