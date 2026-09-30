# ARM CP14 ICEbreaker Debug Communications Channel

Source: https://problemkaputt.de/gbatek.htm#armcp14icebreakerdebugcommunicationschannel

```
The ICEbreaker aka EmbeddedICE module may be found in ARM7TDMI and possibly
also in other ARM processors. The main functionality of the module relies on
external inputs (BREAKPT signal, etc.) being controlled by external debugging
hardware. At software side, ICEbreaker contains a Debug Communications Channel
(again to access external hardware), which can be accessed as coprocessor 14
via following opcodes:

  MRC{cond} P14,0,Rd,C0,C0,0  ;Read Debug Comms Control Register
  MRC{cond} P14,0,Rd,C1,C0,0  ;Read Debug Comms Data Register
  MRC{cond} P14,0,Rd,C2,C0,0  ;Read Debug Comms Status Register
  MCR{cond} P14,0,Rd,C1,C0,0  ;Write Debug Comms Data Register
  MCR{cond} P14,0,Rd,C2,C0,0  ;Write Debug Comms Status Register

The Control register consists of Bit31-28=ICEbreaker version (0001b for
ARM7TDMI), Bit27-2=Not specified, Bit0/Bit1=Data Read/Write Status Flags.

The NDS7 and GBA allow to access CP14 (unlike as for CP0..CP13 & CP15,
access to CP14 doesn't generate any exceptions), however, the ICEbreaker module
appears to be disabled (or completely unimplemented), any reads from
P14,0,Rd,C0,C0,0 through P14,7,Rd,C15,C15,7 are simply returning the prefetched
opcode value from [$+8]. ICEbreaker might be eventually used and enabled in
Nintendo's hardware debuggers, although external breakpoints are reportedly
implemented via /FIQ input rather than via ICEbreaker hardware.

The NDS9 doesn't include a CP14 unit (or it is fully disabled), any attempts to
access it are causing invalid instruction exceptions.
```
