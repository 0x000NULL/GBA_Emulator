# THUMB Instruction Summary

Source: https://problemkaputt.de/gbatek.htm#thumbinstructionsummary

```
The table below lists all THUMB mode instructions with clock cycles, affected
CPSR flags, Format/chapter number, and description.

Only register R0..R7 can be used in thumb mode (unless R8-15,SP,PC are
explicitly mentioned).

Logical Operations

  Instruction           Cycles Flags Format Expl.
  MOV{S} Rd,Imm8bit      1S     NZ--  3   Rd=nn
  MOV{S} Rd,Rs           1S     NZ00  2   Rd=Rs+0       (Pre-UAL)
  CPY    Rd,Rs           1S     ----  5   Rd=Rs         (Pre-UAL, ARMv6)
  MOV    Rd,Rs           1S     ----  5   Rd=Rs         (UAL)
  MOV{S} Rd,Rs           1S     NZ--  2   Rd=Rs SHL 0   (UAL)
  MOV    R0..14,R8..15   1S     ----  5   Rd=Rs
  MOV    R8..14,R0..15   1S     ----  5   Rd=Rs
  MOV    R15,R0..15      2S+1N  ----  5   PC=Rs
  MVN{S} Rd,Rs           1S     NZ--  4   Rd=NOT Rs
  AND{S} Rd,Rs           1S     NZ--  4   Rd=Rd AND Rs
  TST    Rd,Rs           1S     NZ--  4 Void=Rd AND Rs
  BIC{S} Rd,Rs           1S     NZ--  4   Rd=Rd AND NOT Rs
  ORR{S} Rd,Rs           1S     NZ--  4   Rd=Rd OR Rs
  EOR{S} Rd,Rs           1S     NZ--  4   Rd=Rd XOR Rs
  LSL{S} Rd,Rs,Imm5bit   1S     NZc-  1   Rd=Rs SHL nn
  LSL{S} Rd,Rs           1S+1I  NZc-  4   Rd=Rd SHL (Rs AND 0FFh)
  LSR{S} Rd,Rs,Imm5bit   1S     NZc-  1   Rd=Rs SHR nn
  LSR{S} Rd,Rs           1S+1I  NZc-  4   Rd=Rd SHR (Rs AND 0FFh)
  ASR{S} Rd,Rs,Imm5bit   1S     NZc-  1   Rd=Rs SAR nn
  ASR{S} Rd,Rs           1S+1I  NZc-  4   Rd=Rd SAR (Rs AND 0FFh)
  ROR{S} Rd,Rs           1S+1I  NZc-  4   Rd=Rd ROR (Rs AND 0FFh)
  NOP                    1S     ----  5   R8=R8

Carry flag affected only if shift amount is non-zero.

Arithmetic Operations and Multiply

  Instruction           Cycles Flags Format Expl.
  ADD{S} Rd,Rs,Imm3bit   1S     NZCV  2   Rd=Rs+nn
  ADD{S} Rd,Imm8bit      1S     NZCV  3   Rd=Rd+nn
  ADD{S} Rd,Rs,Rn        1S     NZCV  2   Rd=Rs+Rn
  ADD    Rd,Rs           1S     ----  5   Rd=Rd+Rs (UAL only)
  ADD    R0..14,R8..15   1S     ----  5   Rd=Rd+Rs
  ADD    R8..14,R0..15   1S     ----  5   Rd=Rd+Rs
  ADD    R15,R0..15      2S+1N  ----  5   PC=Rd+Rs
  ADD    Rd,PC,Imm8bit*4 1S     ---- 12   Rd=(($+4) AND NOT 2)+nn
  ADD    Rd,SP,Imm8bit*4 1S     ---- 12   Rd=SP+nn
  ADD    SP,Imm7bit*4    1S     ---- 13   SP=SP+nn
  ADD    SP,-Imm7bit*4   1S     ---- 13   SP=SP-nn
  ADC{S} Rd,Rs           1S     NZCV  4   Rd=Rd+Rs+Cy
  SUB{S} Rd,Rs,Imm3Bit   1S     NZCV  2   Rd=Rs-nn
  SUB{S} Rd,Imm8bit      1S     NZCV  3   Rd=Rd-nn
  SUB{S} Rd,Rs,Rn        1S     NZCV  2   Rd=Rs-Rn
  SUB    SP,Imm7bit*4    1S     ---- 13   SP=SP-nn  ;\alias for ADD
  SUB    SP,-Imm7bit*4   1S     ---- 13   SP=SP+nn  ;/with flipped sign
  SBC{S} Rd,Rs           1S     NZCV  4   Rd=Rd-Rs-NOT Cy
  NEG{S} Rd,Rs           1S     NZCV  4   Rd=0-Rs
  CMP    Rd,Imm8bit      1S     NZCV  3 Void=Rd-nn
  CMP    Rd,Rs           1S     NZCV  4 Void=Rd-Rs
  CMP    R0-15,R8-15     1S     NZCV  5 Void=Rd-Rs
  CMP    R8-15,R0-15     1S     NZCV  5 Void=Rd-Rs
  CMN    Rd,Rs           1S     NZCV  4 Void=Rd+Rs
  MUL{S} Rd,Rs           1S+mI  NZx-  4   Rd=Rd*Rs

Jumps and Calls

  Instruction        Cycles    Flags Format Expl.
  B disp              2S+1N     ---- 18  PC=$+/-2048
  BL disp             3S+1N     ---- 19  PC=$+/-4M, LR=$+5
  B{cond=true} disp   2S+1N     ---- 16  PC=$+/-0..256
  B{cond=false} disp  1S        ---- 16  N/A
  BX R0..15           2S+1N     ----  5  PC=Rs, ARM/THUMB (Rs bit0)
  SWI Imm8bit         2S+1N     ---- 17  PC=8, ARM SVC mode, LR=$+2
  BKPT Imm8bit        ???       ---- 17  ??? ARM9 Prefetch Abort
  BLX disp            ???       ---- ??? ??? ARM9
  BLX R0..R14         ???       ---- ??? ??? ARM9
  POP {Rlist,}PC   (n+1)S+2N+1I ---- 14
  MOV R15,R0..15      2S+1N     ----  5  PC=Rs
  ADD R15,R0..15      2S+1N     ----  5  PC=Rd+Rs

The thumb BL instruction occupies two 16bit opcodes, 32bit in total.

Memory Load/Store

  Instruction        Cycles    Flags Format Expl.
  LDR  Rd,[Rb,5bit*4] 1S+1N+1I  ----  9  Rd = WORD[Rb+nn]
  LDR  Rd,[PC,8bit*4] 1S+1N+1I  ----  6  Rd = WORD[PC+nn]
  LDR  Rd,[SP,8bit*4] 1S+1N+1I  ---- 11  Rd = WORD[SP+nn]
  LDR  Rd,[Rb,Ro]     1S+1N+1I  ----  7  Rd = WORD[Rb+Ro]
  LDRB Rd,[Rb,5bit*1] 1S+1N+1I  ----  9  Rd = BYTE[Rb+nn]
  LDRB Rd,[Rb,Ro]     1S+1N+1I  ----  7  Rd = BYTE[Rb+Ro]
  LDRH Rd,[Rb,5bit*2] 1S+1N+1I  ---- 10  Rd = HALFWORD[Rb+nn]
  LDRH Rd,[Rb,Ro]     1S+1N+1I  ----  8  Rd = HALFWORD[Rb+Ro]
  LDSB Rd,[Rb,Ro]     1S+1N+1I  ----  8  Rd = SIGNED_BYTE[Rb+Ro]
  LDSH Rd,[Rb,Ro]     1S+1N+1I  ----  8  Rd = SIGNED_HALFWORD[Rb+Ro]
  STR  Rd,[Rb,5bit*4] 2N        ----  9  WORD[Rb+nn] = Rd
  STR  Rd,[SP,8bit*4] 2N        ---- 11  WORD[SP+nn] = Rd
  STR  Rd,[Rb,Ro]     2N        ----  7  WORD[Rb+Ro] = Rd
  STRB Rd,[Rb,5bit*1] 2N        ----  9  BYTE[Rb+nn] = Rd
  STRB Rd,[Rb,Ro]     2N        ----  7  BYTE[Rb+Ro] = Rd
  STRH Rd,[Rb,5bit*2] 2N        ---- 10  HALFWORD[Rb+nn] = Rd
  STRH Rd,[Rb,Ro]     2N        ----  8  HALFWORD[Rb+Ro]=Rd
  PUSH {Rlist}{LR}    (n-1)S+2N ---- 14
  POP  {Rlist}{PC}              ---- 14  (ARM9: with mode switch)
  STMIA Rb!,{Rlist}   (n-1)S+2N ---- 15
  LDMIA Rb!,{Rlist}   nS+1N+1I  ---- 15

THUMB Binary Opcode Format

This table summarizes the position of opcode/parameter bits for THUMB mode
instructions, Format 1-19.

 Form|_15|_14|_13|_12|_11|_10|_9_|_8_|_7_|_6_|_5_|_4_|_3_|_2_|_1_|_0_|
 __1_|_0___0___0_|__Op___|_______Offset______|____Rs_____|____Rd_____|Shifted
 __2_|_0___0___0___1___1_|_I,_Op_|___Rn/nn___|____Rs_____|____Rd_____|ADD/SUB
 __3_|_0___0___1_|__Op___|____Rd_____|_____________Offset____________|Immedi.
 __4_|_0___1___0___0___0___0_|______Op_______|____Rs_____|____Rd_____|AluOp
 __5_|_0___1___0___0___0___1_|__Op___|Hd_|Hs_|____Rs_____|____Rd_____|HiReg/BX
 __6_|_0___1___0___0___1_|____Rd_____|_____________Word______________|LDR PC
 __7_|_0___1___0___1_|__Op___|_0_|___Ro______|____Rb_____|____Rd_____|LDR/STR
 __8_|_0___1___0___1_|__Op___|_1_|___Ro______|____Rb_____|____Rd_____|""H/SB/SH
 __9_|_0___1___1_|__Op___|_______Offset______|____Rb_____|____Rd_____|""{B}
 _10_|_1___0___0___0_|Op_|_______Offset______|____Rb_____|____Rd_____|""H
 _11_|_1___0___0___1_|Op_|____Rd_____|_____________Word______________|"" SP
 _12_|_1___0___1___0_|Op_|____Rd_____|_____________Word______________|ADD PC/SP
 _13_|_1___0___1___1___0___0___0___0_|_S_|___________Word____________|ADD SP,nn
 _14_|_1___0___1___1_|Op_|_1___0_|_R_|____________Rlist______________|PUSH/POP
 _17_|_1___0___1___1___1___1___1___0_|___________User_Data___________|ARM9:BKPT
 _X__|_1___0___1___1___   ..............................             |ARM11...
  0110 011 Change Processor State        CPS on page B4-2
  0001 xxx Compare and Branch on Zero    CBNZ, CBZ on page A6-52
  1011 xxx Compare and Branch on Nonzero CBNZ, CBZ on page A6-52
  0011 xxx Compare and Branch on Zero    CBNZ, CBZ on page A6-52
  1001 xxx Compare and Branch on Nonzero CBNZ, CBZ on page A6-52
  0010 00x Signed Extend Halfword        SXTH on page A6-256
  0010 01x Signed Extend Byte            SXTB on page A6-254
  0010 10x Unsigned Extend Halfword      UXTH on page A6-274
  0010 11x Unsigned Extend Byte          UXTB on page A6-272
  1010 00x Byte-Reverse Word             REV on page A6-191
  1010 01x Byte-Reverse Packed Halfword  REV16 on page A6-192
  1010 11x Byte-Reverse Signed Halfword  REVSH on page A6-193
  1111 xxx If-Then, and hints If-Then, and hints on page A5-11
 _15_|_1___1___0___0_|Op_|____Rb_____|____________Rlist______________|STM/LDM
 _16_|_1___1___0___1_|_____Cond______|_________Signed_Offset_________|B{cond}
 _U__|_1___1___0___1___1___1___1___0_|_____________var_______________|UndefARM9
 _17_|_1___1___0___1___1___1___1___1_|___________User_Data___________|SWI
 _18_|_1___1___1___0___0_|________________Offset_____________________|B
 _19_|_1___1___1___0___1_|_________________________var___________|_0_|BLX.ARM9
 _U__|_1___1___1___0___1_|_________________________var___________|_1_|UndefARM9
 _19_|_1___1___1___1_|_H_|______________Offset_Low/High______________|BL,BLX

Further UNDEFS ??? ARM9?

 1011 0001 xxxxxxxx (reserved)
 1011 0x1x xxxxxxxx (reserved)
 1011 10xx xxxxxxxx (reserved)
 1011 1111 xxxxxxxx (reserved)
 1101 1110 xxxxxxxx (free for user)
```
