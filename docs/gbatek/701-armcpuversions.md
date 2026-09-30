# ARM CPU Versions

Source: https://problemkaputt.de/gbatek.htm#armcpuversions

```
Version Numbers

ARM CPUs are distributed by name ARM#, and are described as ARMv# in
specifications, whereas "#" is NOT the same than "v#", for example, ARM7TDMI is
ARMv4TM. That is so confusing, that ARM didn't even attempt to clarify the
relationship between the various "#" and "v#" values.

Version Variants

Suffixes like "M" (long multiply), "T" (Thumb support), "E" (Enhanced DSP)
indicate presence of special features, additionally to the standard instruction
set of a given version, or, when preceded by an "x", indicate the absence of
that features.

ARMv1 aka ARM1

Some sort of a beta version, according to ARM never been used in any commercial
products.

ARMv2 and up

MUL,MLA

CDP,LDC,MCR,MRC,STC

SWP/SWPB (ARMv2a and up only)

Two new FIQ registers

ARMv3 and up

MRS,MSR opcodes (instead CMP/CMN/TST/TEQ{P} opcodes)

CPSR,SPSR registers (instead PSR bits in R15)

Removed never condition, cond=NV no longer valid

32bit addressing (instead 26bit addressing in older versions)

26bit addressing backwards comptibility mode (except v3G)

Abt and Und modes (instead handling aborts/undefined in Svc mode)

SMLAL,SMULL,UMLAL,UMULL (optionally, INCLUDED in v3M, EXCLUDED in v4xM/v5xM)

ARMv4 aka ARM7 and up

LDRH,LDRSB,LDRSH,STRH

Sys mode (privileged user mode)

BX (only ARMv4T, and any ARMv5 or ARMv5T and up)

THUMB code (only T variants, ie. ARMv4T, ARMv5T)

ARMv5 aka ARM9 and up

BKPT,BLX,CLZ (BKPT,BLX also in THUMB mode)

LDM/LDR/POP PC with mode switch (POP PC also in THUMB mode)

CDP2,LDC2,MCR2,MRC2,STC2 (new coprocessor opcodes)

C-flag unchanged by MUL (instead undefined flag value)

changed instruction cycle timings / interlock ??? or not ???

QADD,QDADD,QDSUB,QSUB opcodes, CPSR.Q flag (v5TE and V5TExP only)

SMLAxy,SMLALxy,SMLAWy,SMULxy,SMULWy (v5TE and V5TExP only)

LDRD,STRD,PLD,MCRR,MRRC (v5TE only, not v5, not v5TExP)

BXJ (jump to Jazelle bytecode) (v5TEJ only)

ARMv6 aka ARM11 and up (see ARM DDI 0100I)

Supports all ARMv5TEJ features (=including Jazelle?).

The following ARM instructions are added:

- CPS, SRS and RFE instructions for improved exception handling

- REV, REV16 and REVSH byte reversal instructions

- SETEND for a revised endian (memory) model

- LDREX and STREX exclusive access instructions

- SXTB, SXTH, UXTB, UXTH byte/halfword extend instructions

- A set of Single Instruction Multiple Data (SIMD) media instructions

- Additional forms of multiply with accumulation into a 64-bit result

The following Thumb instructions are added:

- CPS, CPY (a form of MOV), REV, REV16, REVSH, SETEND, SXTB, SXTH, UXTB, UXTH

SWP/SWPB has been deprecated in ARMv6, made optional in ARMv7 (with the
possibility of disabling it if still available), and removed in ARMv8.

ARMv6K aka "ARM11 MPCore" (see ARM DDI 0360F) (used in 3DS)

Unknown how this is abbreviated (ARMv6-M would be something else: Cortex-M).

The MPCore processor provides support for extensions to ARMv6 that include:

- LDREX/STREX{B|H|D} for 8bit/16bit/64bit and CLREX

- HINT (TrueNOP, YIELD, WFI, WFE, SEV)

- Architectural remap registers.

- Revised use of TEX remap bits. The ARMv6 MMU page table descriptors use a
large number of bits to describe all of the options for inner and outer
cachability. In reality, no application requires all of these options
simultaneously. Therefore it is possible to use the TEX remap mechanism to
configure the MP11 CPUs to support only a small number of options. This implies
a level of indirection in the page table mappings.

The TEX CB encoding table provides two OS managed page table bits. For binary
compatibility with existing ARMv6 ports of OSs, this gives a separate mode of
operation of the MMU. This is called the TEX Remap configuration and is
controlled by bit [28] TR in CP15 Register 1.

- Revised use of AP bits. In the MP11 CPUs the APX and AP[1:0] encoding b111 is
Privileged or User mode read-only access. AP[0] indicates an abort type, Access
Bit fault, when CP15 c1[29] is 1.

For more information see the ARM Architecture Reference Manual.

ARMv6T2 aka THUMB-2

This doesn't seem to be supported in 3DS. THUMB-2 adds more 2x16bit opcodes in
THUMB mode, ie. THUMB code can contain classic 16bit opcodes, mixed with new
2x16bit opcodes. As far as I understand, the old 2x16bit "BL" and "BLX" do no
longer exists, or are replaced by differently encode THUMB-2 opcodes?

ARMv6Z

ARMv6K with Security Extensions. Aka TrustZone, see Processor Feature Register
1.

A Milestone in Computer History

Original ARMv2 has been used in the relative rare and expensive Archimedes
deluxe home computers in the late eighties, the Archimedes has caught a lot of
attention, particularly for being the first home computer that used a BIOS
being programmed in BASIC language - which has been a absolutely revolutionary
decadency at that time.

Inspired, programmers all over the world have successfully developed even
slower and much more inefficient programming languages, which are nowadays
consequently used by nearly all ARM programmers, and by most non-ARM
programmers as well.
```
