# BIOS Arithmetic Functions

Source: https://problemkaputt.de/gbatek.htm#biosarithmeticfunctions

```
Div

DivArm

Sqrt

ArcTan

ArcTan2

SWI 06h (GBA) or SWI 09h (NDS7/NDS9/DSi7/DSi9) - Div

Signed Division, r0/r1.

  r0  signed 32bit Number
  r1  signed 32bit Denom

Return:

  r0  Number DIV Denom ;signed
  r1  Number MOD Denom ;signed
  r3  ABS (Number DIV Denom) ;unsigned

For example, incoming -1234, 10 should return -123, -4, +123.

The function usually gets caught in an endless loop upon division by zero.

Note: The NDS9 and DSi9 additionally support hardware division, by math
coprocessor, accessed via I/O Ports, however, the SWI function is a raw
software division.

SWI 07h (GBA) - DivArm

Same as above (SWI 06h Div), but incoming parameters are exchanged, r1/r0
(r0=Denom, r1=number). For compatibility with ARM's library. Slightly slower (3
clock cycles) than SWI 06h.

SWI 08h (GBA) or SWI 0Dh (NDS7/NDS9/DSi7/DSi9) - Sqrt

Calculate square root.

  r0   unsigned 32bit number

Return:

  r0   unsigned 16bit number

The result is an integer value, so Sqrt(2) would return 1, to avoid this
inaccuracy, shift left incoming number by 2*N as much as possible (the result
is then shifted left by 1*N). Ie. Sqrt(2 shl 30) would return 1.41421 shl 15.

Note: The NDS9 and DSi9 additionally support hardware square root calculation,
by math coprocessor, accessed via I/O Ports, however, the SWI function is a raw
software calculation.

SWI 09h (GBA) - ArcTan

Calculates the arc tangent.

  r0   Tan, 16bit (1bit sign, 1bit integral part, 14bit decimal part)

Return:

  r0   "-PI/2<THETA/<PI/2" in a range of C000h-4000h.

Note: there is a problem in accuracy with "THETA<-PI/4, PI/4<THETA".

SWI 0Ah (GBA) - ArcTan2

Calculates the arc tangent after correction processing.

Use this in normal situations.

  r0   X, 16bit (1bit sign, 1bit integral part, 14bit decimal part)
  r1   Y, 16bit (1bit sign, 1bit integral part, 14bit decimal part)

Return:

  r0   0000h-FFFFh for 0<=THETA<2PI.
```
