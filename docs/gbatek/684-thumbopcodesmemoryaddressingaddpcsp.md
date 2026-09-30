# THUMB Opcodes: Memory Addressing (ADD PC/SP)

Source: https://problemkaputt.de/gbatek.htm#thumbopcodesmemoryaddressingaddpcsp

```
THUMB.12: get relative address

  15-12  Must be 1010b for this type of instructions
  11     Opcode/Source Register (0-1)
          0: ADD  Rd,PC,#nn    ;Rd = (($+4) AND NOT 2) + nn
          1: ADD  Rd,SP,#nn    ;Rd = SP + nn
  10-8   Rd - Destination Register         (R0..R7)
  7-0    nn - Unsigned Offset              (0-1020, step 4)

Return: No flags affected, result in Rd.

Execution Time: 1S

THUMB.13: add offset to stack pointer

  15-8   Must be 10110000b for this type of instructions
  7      Opcode/Sign
          0: ADD  SP,#nn       ;SP = SP + nn
          1: ADD  SP,#-nn      ;SP = SP - nn
          1: SUB  SP,#nn       ;SP = SP - nn  ;\alias for ADD
          0: SUB  SP,#-nn      ;SP = SP + nn  ;/with flipped sign
  6-0    nn - Unsigned Offset    (0-508, step 4)

Return: No flags affected, SP adjusted.

Execution Time: 1S
```
