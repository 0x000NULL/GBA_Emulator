# ARM Classic vs UAL Syntax (Unified Assembler Language)

Source: https://problemkaputt.de/gbatek.htm#armclassicvsualsyntaxunifiedassemblerlanguage

```
ARM has changed the ARM/THUMB syntax (apparently at some point between ARMv6
and ARMv6K). The new UAL syntax contains some cosmetic changes, and fixes some
serious flaws in the old THUMB syntax.

GBA/NDS software did traditionally use old syntax. DSi software probably, too.
3DS software uses ARMv6K, so it's more likely to use the new UAL syntax.

THUMB code with {S} suffix

Most ALU opcodes do always affect flags in THUMB mode, although they were
written without {S} suffix in Pre-UAL syntax, which is problematic because the
same source code can behave differently for ARM and THUMB.

MOV/ADD/SUB don't affect flags when using SP, PC, or R8..R15 as source/dest
operands.

MOV/ADD have variants that don't affect flags for operands R0..R7 (these
variants were originally undocumented/reserved; the flag-less CPY variant was
officially allowed in ARMv6, the flag-less ADD seems to be allowed (?) in UAL).

MOV is a total mess because it relies on poorly defined pseudo opcodes (in the
Pre-UAL syntax it was common to encode it as ADD{S} opcode, which destroyed the
C,V flags).

  Pre-UAL               UAL
  MOV Rd,Imm            MOVS Rd,Imm                ;\
  LSL with R0..R7,0     MOVS with R0..R7           ;
  MOV with R0..R7       ADDS Rd,Rs,0               ; move
  MOV with R8..R15      MOV  with R8..R15          ;
  CPY with R0..R7       MOV  with R0..R7           ;
  SHIFT Rd,..,s         MOVS Rd,..,SHIFT s         ;/
  ADD with R0..R7       ADDS with R0..R7           ;\
  ADD with R8..R15      ADD  with R8..R15          ; add
  N/A (reserved)        ADD  with R0..R7           ;/
  SUB with R0..R7       SUBS with R0..R7           ;\subtract
  SUB SP,Imm            SUB SP,Imm  (aka ADD -Imm) ;/
  ADC/SBC/NEG/RSB/MUL   ADCS/SBCS/NEGS/RSBS/MULS   ;\
  AND/ORR/EOR/BIC/MVN   ANDS/ORRS/EORS/BICS/MVNS   ; always affects flags
  LSL/LSR/ASR/ROR       LSLS/LSRS/ASRS/RORS        ;/

These changes are solving compatibility issues between ARM and THUMB code.
However, in case of MOV and ADD they do bring up compatibility issues between
old and new THUMB code.

Renamed Opcodes

  Pre-UAL               UAL
  LDR{cond}{xx}         LDR{xx}{cond}              ;\
  ADD{cond}{S}          ADD{S}{cond}               ; cond suffix moved to end
  xxx{cond}{xx}         xxx{xx}{cond}              ;/
  LDMIA                 LDM                        ;\amod IA can be ommited
  STMIA                 STM                        ;/
  LDRD Rd,[addr]        LDRD Rd,Rd+1,[addr]        ;\double word load/store
  STRD Rd,[addr]        STRD Rd,Rd+1,[addr]        ;/
  SWI (software irq)    SVC (supervisior)          ;-renamed SWI to SVC

Alternate Opcode Names

Some opcodes had different names in classic ARM/THUMB syntax, UAL syntax allows
to use either one in both ARM and THUMB mode.

  THUMB-style           ARM-style
  PUSH rlist            STMFD r13!,rlist           ;\PUSH/POP
  POP rlist             LDMFD r13!,rlist           ;/
  NEG{S} Rd,Rs          RSB{S} Rd,Rs,0             ;-NEG/RSB
  LSL{S} Rd,Rn,shift    MOV{S} Rd,Rn,LSL shift     ;\
  LSR{S} Rd,Rn,shift    MOV{S} Rd,Rn,LSR shift     ;
  ASR{S} Rd,Rn,shift    MOV{S} Rd,Rn,ASR shift     ; MOV/SHIFT
  ROR{S} Rd,Rn,shift    MOV{S} Rd,Rn,ROR shift     ;
  RRX{S} Rd,Rn          MOV{S} Rd,Rn,RRX           ;/
  CMD{S} Rd,Op2         CMD{S} Rd,Rd,Op2           ;-long/short forms

Further UAL changes

  NOP: uses new dedicated NOP opcodes (instead of MOV Rd,Rd pseudo opcodes)
  LDMIA: should omit "!" if Rn in rlist
  LDR Rd,[pc,#offset]: should be somehow written as LDR Rd,label
  Newer opcodes (for ARMv6K and up) seem to exist only in UAL syntax

Moreover, unrelated to UAL syntax, the 4-letter LDSB/LDSH/STSB/STSH THUMB
opcodes seem have been renamed to LDRSB/LDRSH/STRSB/STRSH at some point. And,
SUB SP,imm seems to have been added as an official THUMB alias for ADD SP,-imm.
```
