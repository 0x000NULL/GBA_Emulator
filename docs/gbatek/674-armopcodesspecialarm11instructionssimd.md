# ARM Opcodes: Special ARM11 Instructions (SIMD)

Source: https://problemkaputt.de/gbatek.htm#armopcodesspecialarm11instructionssimd

```
Sign Extend / Zero Extend, with optional Addition (SXT, UXT)

  31-28  Condition
  27-23  Fixed, 01101b
  22-20  Opcode:
          0: SXTAB16{cond} Rd, Rn, Rm{,ROR #imm} ;\
          2: SXTAB{cond}   Rd, Rn, Rm{,ROR #imm} ;
          3: SXTAH{cond}   Rd, Rn, Rm{,ROR #imm} ; with add
          4: UXTAB16{cond} Rd, Rn, Rm{,ROR #imm} ;
          6: UXTAB{cond}   Rd, Rn, Rm{,ROR #imm} ;
          7: UXTAH{cond}   Rd, Rn, Rm{,ROR #imm} ;/
          0: SXTB16{cond}  Rd,     Rm{,ROR #imm} ;\
          2: SXTB{cond}    Rd,     Rm{,ROR #imm} ;
          3: SXTH{cond}    Rd,     Rm{,ROR #imm} ; without add
          4: UXTB16{cond}  Rd,     Rm{,ROR #imm} ;
          6: UXTB{cond}    Rd,     Rm{,ROR #imm} ;
          7: UXTH{cond}    Rd,     Rm{,ROR #imm} ;/
  19-16  Rn - Addition Register (or 1111b when no addition)            (R0-R14)
  15-12  Rd - Destination Register                                     (R0-R14)
  11-10  Rotation (0=None/ROR#0, 1=ROR#8, 2=ROR#16, 3=ROR#24)
  9-4    Fixed, 000111b
  3-0    Rm - Source register (to be rotated/extracted from)           (R0-R14)

Supported on ARMv6 and above.

Select Bytes from 1st/2nd operand, depending on GE flags (SEL)

Syntax

  SEL{cond} Rd, Rn, Rm

Operation

if ConditionPassed(cond) then

Rd[7:0] = if GE[0] == 1 then Rn[7:0] else Rm[7:0]

Rd[15:8] = if GE[1] == 1 then Rn[15:8] else Rm[15:8]

Rd[23:16] = if GE[2] == 1 then Rn[23:16] else Rm[23:16]

Rd[31:24] = if GE[3] == 1 then Rn[31:24] else Rm[31:24]

Usage

Use SEL after instructions such as SADD8, SADD16, SSUB8, SSUB16, UADD8, UADD16,
USUB8, USUB16, SADDSUBX, SSUBADDX, UADDSUBX and USUBADDX, that set the GE
flags. For example, the following sequence of instructions sets each byte of Rd
equal to the unsigned minimum of the corresponding bytes of Ra and Rb:

  USUB8 Rd, Ra, Rb
  SEL Rd, Rb, Ra

Notes

Specifying R15 for register Rd, Rm, or Rn has UNPREDICTABLE results.

  31-28  Condition
  27-20  Fixed, 01101000b
  19-16  Rn - First Source Register (8bit snippets used when GE bits = 1)
  15-12  Rd - Destination Register
  11-4   Fixed, 11111011b
  3-0    Rm - Second Source Register (8bit snippets used when GE bits = 0)

ARMv6 and above.

SADD16

SADD16 (Signed Add) performs two 16-bit signed integer additions. It sets the
GE bits in the CPSR according to the results of the additions.

ARMv6 and above.

Operation

sum = Rn[15:0] + Rm[15:0] /* Signed addition */

Rd[15:0] = sum[15:0]

GE[1:0] = if sum >= 0 then 0b11 else 0

sum = Rn[31:16] + Rm[31:16] /* Signed addition */

Rd[31:16] = sum[15:0]

GE[3:2] = if sum >= 0 then 0b11 else 0

  31-28  Condition
  27-23  Fixed, 01100b
  22-20  Opcode, part 1:
          001b: for Sxx           (signed, with GE flags)
          010b: for Qxx           (signed, with saturation)
          011b: for SHxx          (signed, with halving)
          101b: for Uxx           (unsigned, with GE flags)
          110b: for UQxx          (unsigned, with saturation)
          111b: for UHxx          (unsigned, with halving)
  19-16  Rn - First Source Register                                    (R0-R14)
  15-12  Rd - Destination Register                                     (R0-R14)
  11-8   Fixed, 1111b
  7-5    Opcode, part 2:
          000b: xxADD16{cond}   Rd, Rn, Rm
          001b: xxADDSUBX{cond} Rd, Rn, Rm
          010b: xxSUBADDX{cond} Rd, Rn, Rm
          011b: xxSUB16{cond}   Rd, Rn, Rm
          100b: xxADD8{cond}    Rd, Rn, Rm
          111b: xxSUB8{cond}    Rd, Rn, Rm
  4      Fixed, 1
  3-0    Rm - Second Source register                                   (R0-R14)

SMLAD

SMLAD (Signed Multiply Accumulate Dual) performs two signed 16 x 16-bit
multiplications. It adds the products to a 32-bit accumulate operand.

Optionally, you can exchange the halfwords of the second operand before
performing the arithmetic. This produces top x bottom and bottom x top
multiplication.

This instruction sets the Q flag if the accumulate operation overflows.
Overflow cannot occur during the multiplications.

X Sets the X bit of the instruction to 1, and the multiplications are bottom x
top and top x bottom.

If the X is omitted, sets the X bit to 0, and the multiplications are bottom x
bottom and top x top.

<Rm> Specifies the register that contains the first operand.

<Rs> Specifies the register that contains the second operand.

<Rn> Specifies the register that contains the accumulate operand.

  31-28  Condition
  27-23  Fixed, 01110
  22-20  Opcode part 1 (part2 in bit7-6)
         Merged part1.part2:
           000.00: SMUAD{X}{cond}  Rd,Rm,Rs
           000.00: SMLAD{X}{cond}  Rd,Rm,Rs,Rn
           000.01: SMUSD{X}{cond}  Rd,Rm,Rs
           000.01: SMLSD{X}{cond}  Rd,Rm,Rs,Rn
           100.00: SMLALD{X}{cond} RdLo,RdHi,Rm,Rs
           100.01: SMLSLD{X}{cond} RdLo,RdHi,Rm,Rs
           101.00: SMMUL{R}{cond}  Rd,Rm,Rs        ;Rd=(Rm*Rs)/100000000h
           101.00: SMMLA{R}{cond}  Rd,Rm,Rs,Rn     ;Rd=(Rm*Rs)/100000000h+Rz
           101.11: SMMLS{R}{cond}  Rd,Rm,Rs,Rn     ;Rd=(Rm*Rs)/100000000h-Rz
  19-16  Rd or RdHi - Destination Register
  15-12  Rn or RdLo or unused (must be 0Fh for SMUAD/SMUSD/SMMUL)
  11-8   Rs
  7-6    Opcode part 2 (part1 in bit22-20, see there)
  5      X or R flag   (X=Exchange, R=Rounded)
  4      Fixed, 1
  3-0    Rm

ARMv6 and above.

Most significant word multiply

There are three signed 32-bit x 32-bit Multiply instructions that produce top
32-bit results:

  SMMUL Multiplies the 32-bit values of two registers together, and stores
        the top 32 bits of the signed 64-bit result in a third register.
  SMMLA Multiplies the 32-bit values of two registers together, extracts the
        top 32 bits, adds the 32-bit value from a third register, and stores
        the signed 32-bit result in a fourth register.
  SMMLS Multiplies the 32-bit value of two registers together, extracts the
        top 32 bits, subtracts this from a 32-bit value from a third
        register, and stores the signed 32-bit result in a fourth register.

The above three instructions do not affect any flags.

Dual halfword multiply

There are six dual, signed 16-bit x 16-bit Multiply instructions:

  SMUAD Multiplies the values of the top halfwords of two registers together,
        multiplies the values of the bottom halfwords of the same two
        registers together, adds the products, and stores the 32-bit result
        in a third register.
  SMUSD Multiplies the values of the top halfwords of two registers together,
        multiplies the values of the bottom halfwords of the same two
        registers together, subtracts one product from the other, and stores
        the 32-bit result in a third register.
  ---(descriptions below seem to be bugged, seem to describe 32bit SMMxx)---
  SMLAD Multiplies the 32-bit value of two registers together, extracts the
        top 32 bits, subtracts this from a 32-bit value from a third
        register, and stores the signed 32-bit result in a fourth register.
  SMLSD Multiplies the 32-bit values of two registers together, extracts the
        top 32 bits, adds the 32-bit value from a third register, and stores
        the signed 32-bit result in a fourth register.
  SMLALD Multiplies the 32-bit value of two registers together, extracts the
         top 32 bits, subtracts this from a 32-bit value from a third
         register, and stores the signed 32-bit result in a fourth register.
  SMLSLD Multiplies the 32-bit value of two registers together, extracts the
         top 32 bits, subtracts this from a 32-bit value from a third
         register, and stores the signed 32-bit result in a fourth register.

SMUAD, SMLAD, and SMLSLD can set the Q flag if overflow occurs in the
operation. All other instructions do not affect any flags.

Signed/Unsigned Saturate (SSAT, USAT)

Saturates a 32bit value (with optional shift) to the specified range (of 1..32
bits).

  31-28  Condition
  27-21  Fixed, 0110101b: SSAT{cond} Rd, #<range>, Rm {,LSL/ASR #imm}
                0110111b: USAT{cond} Rd, #<range>, Rm {,LSL/ASR #imm}
  20-16  Immediate: range-1 (0..31 = Saturate to 1..32 bits range)
  15-12  Rd - Destination Register
  11-7   shift_imm (1..31 for LSL/ASR#1..31, or 0 for LSL#0, or 0 for ASR#32)
  6      Shift type (0=LSL, 1=ASR) (note: really ASR, even "unsigned" USAT)
  5-4    Fixed, 01b
  3-0    Rm - Source Register

The Q flag is set if the operation saturates.

ARMv6 and above.

Signed/Unsigned Saturate on Halfwords (SSAT16, USAT16)

Saturates two 16bit values to the specified range (of 1..16 bits).

  31-28  Condition
  27-21  Fixed, 0110101b: SSAT16{cond} Rd, #<range>, Rm
                0110111b: USAT16{cond} Rd, #<range>, Rm
  20     Fixed, 0
  19-16  Immediate: range-1 (0..15 = Saturate to 1..16 bits range)
  15-12  Rd - Destination Register
  11-4   Fixed, 11110011b
  3-0    Rm - Source Register

The Q flag is set if either halfword operation saturates.

ARMv6 and above.

Unsigned Sum of Absolute Differences, optional Accumulate (USAD8, USADA8)

Performs four unsigned 8bit subtractions, and adds the absolute values of the
differences together.

  31-28  Condition
  27-20  Fixed, 01111000b
  19-16  Rd - Destination Register
  15-12  Rn (if any) (otherwise fixed, 1111)
           USAD8{cond}  Rd, Rm, Rs        ;without Rn   ;Rd=sum
           USADA8{cond} Rd, Rm, Rs, Rn    ;with Rn      ;Rd=sum + Rn
  11-8   Rs
  7-4    Fixed, 0001b
  3-0    Rm

ARMv6 and above.

Pack Halfword, Bottom-Top or Top-Bottom (PKHBT, PKHTB)

PKHBT (Pack Halfword Bottom Top) combines the bottom (least significant)
halfword of its first operand with the top (most significant) halfword of its
shifted second operand. The shift is a left shift, by any amount from 0 to 31.

<Rn> Specifies the register that contains the first operand. Bits[15:0]
of this operand become bits[15:0] of the result of the operation.

<Rm> Specifies the register that contains the second operand. This is
shifted left by the specified amount, then bits[31:16] of this operand become
bits[31:16] of the result of the operation.

<shift_imm> Specifies the amount by which Rm is to be shifted left. This
is a value from 0 to 31.

If the shift specifier is omitted, a left shift by 0 is used.

Operation

if ConditionPassed(cond) then

Rd[15:0] = Rn[15:0]

Rd[31:16] = (Rm Logical_Shift_Left shift_imm)[31:16]

  31-28  Condition
  27-20  Fixed, 01101000b
  19-16  Rn
  15-12  Rd - Destination Register
  11-7   shift_imm (0..31)
  6-4    Fixed, 001  PKHBT {cond} Rd, Rn, Rm {, LSL #<shift_imm>}
                101  PKHTB {cond} Rd, Rn, Rm {, ASR #<shift_imm>}
  3-0    Rm

ARMv6 and above.

Reverse Byte Order for Big/Little-Endian Conversion (REV, REV16, REVSH)

Specifying R15 for register Rd or Rm has UNPREDICTABLE results.

  31-28  Condition
  27-23  Fixed, 01101b
  22     Opcode (with bit7)
          Bit22=0, Bit7=0: REV{cond}   Rd, Rm   ;reverse 32bit
          Bit22=0, Bit7=1: REV16{cond} Rd, Rm
          Bit22=1, Bit7=1: REVSH{cond} Rd, Rm
  21-16  Fixed, 111111b
  15-12  Rd - Destination Register
  11-8   Fixed, 1111b
  7      Opcode (with bit22, see there)
  6-4    Fixed, 011b
  3-0    Rm - Source Register

ARMv6 and above.
```
