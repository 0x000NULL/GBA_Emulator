# GBA Sound Control Registers

Source: https://problemkaputt.de/gbatek.htm#gbasoundcontrolregisters

```
4000080h - SOUNDCNT_L (NR50, NR51) - Channel L/R Volume/Enable (R/W)

  Bit        Expl.
  0-2   R/W  Sound 1-4 Master Volume RIGHT (0-7)
  3     -    Not used
  4-6   R/W  Sound 1-4 Master Volume LEFT (0-7)
  7     -    Not used
  8-11  R/W  Sound 1-4 Enable Flags RIGHT (each Bit 8-11, 0=Disable, 1=Enable)
  12-15 R/W  Sound 1-4 Enable Flags LEFT (each Bit 12-15, 0=Disable, 1=Enable)

4000082h - SOUNDCNT_H (GBA only) - DMA Sound Control/Mixing (R/W)

  Bit        Expl.
  0-1   R/W  Sound # 1-4 Volume   (0=25%, 1=50%, 2=100%, 3=Prohibited)
  2     R/W  DMA Sound A Volume   (0=50%, 1=100%)
  3     R/W  DMA Sound B Volume   (0=50%, 1=100%)
  4-7   -    Not used
  8     R/W  DMA Sound A Enable RIGHT (0=Disable, 1=Enable)
  9     R/W  DMA Sound A Enable LEFT  (0=Disable, 1=Enable)
  10    R/W  DMA Sound A Timer Select (0=Timer 0, 1=Timer 1)
  11    W?   DMA Sound A Reset FIFO   (1=Reset)
  12    R/W  DMA Sound B Enable RIGHT (0=Disable, 1=Enable)
  13    R/W  DMA Sound B Enable LEFT  (0=Disable, 1=Enable)
  14    R/W  DMA Sound B Timer Select (0=Timer 0, 1=Timer 1)
  15    W?   DMA Sound B Reset FIFO   (1=Reset)

4000084h - SOUNDCNT_X (NR52) - Sound on/off (R/W)

Bits 0-3 are automatically set when starting sound output, and are
automatically cleared when a sound ends. (Ie. when the length expires, as far
as length is enabled. The bits are NOT reset when an volume envelope ends.)

  Bit        Expl.
  0     R    Sound 1 ON flag (Read Only)
  1     R    Sound 2 ON flag (Read Only)
  2     R    Sound 3 ON flag (Read Only)
  3     R    Sound 4 ON flag (Read Only)
  4-6   -    Not used
  7     R/W  PSG/FIFO Master Enable (0=Disable, 1=Enable) (Read/Write)
  8-31  -    Not used

While Bit 7 is cleared, both PSG and FIFO sounds are disabled, and all PSG
registers at 4000060h..4000081h are reset to zero (and must be re-initialized
after re-enabling sound). However, registers 4000082h and 4000088h are kept
read/write-able (of which, 4000082h has no function when sound is off, whilst
4000088h does work even when sound is off).

4000088h - SOUNDBIAS - Sound PWM Control (R/W, see below)

This register controls the final sound output. The default setting is 0200h, it
is normally not required to change this value.

  Bit        Expl.
  0     -    Not used
  1-9   R/W  Bias Level (Default=100h, converting signed samples into unsigned)
  10-13 -    Not used
  14-15 R/W  Amplitude Resolution/Sampling Cycle (Default=0, see below)
  16-31 -    Not used

Amplitude Resolution/Sampling Cycle (0-3):

  0  9bit / 32.768kHz   (Default, best for DMA channels A,B)
  1  8bit / 65.536kHz
  2  7bit / 131.072kHz
  3  6bit / 262.144kHz  (Best for PSG channels 1-4)

The Bias Level has no audible effect on 3DS in GBA mode.

For more information on this register, read the descriptions below.

400008Ch - Not used

400008Eh - Not used

Max Output Levels (with max volume settings)

Each of the two FIFOs can span the FULL output range (+/-200h).

Each of the four PSGs can span one QUARTER of the output range (+/-80h).

The current output levels of all six channels are added together by hardware.

So together, the FIFOs and PSGs, could reach THRICE the range (+/-600h).

The BIAS value is added to that signed value. With default BIAS (200h), the
possible range becomes -400h..+800h, however, values that exceed the unsigned
10bit output range of 0..3FFh are clipped to MinMax(0,3FFh).

Resampling to 32.768kHz / 9bit (default)

The PSG channels 1-4 are internally generated at 262.144kHz, and DMA sound A-B
could be theoretically generated at timer rates up to 16.78MHz. However, the
final sound output is resampled to a rate of 32.768kHz, at 9bit depth (the
above 10bit value, divided by two). If necessary, rates higher than 32.768kHz
can be selected in the SOUNDBIAS register, that would result in a depth smaller
than 9bit though.

PWM (Pulse Width Modulation) Output 16.78MHz / 1bit

Okay, now comes the actual output. The GBA can output only two voltages (low
and high), these 'bits' are output at system clock speed (16.78MHz). If using
the default 32.768kHz sampling rate, then 512 bits are output per sample
(512*32K=16M). Each sample value (9bit range, N=0..511), would be then output
as N low bits, followed by 512-N high bits. The resulting 'noise' is smoothed
down by capacitors, by the speaker, and by human hearing, so that it will
effectively sound like clean D/A converted 9bit voltages at 32kHz sampling
rate.

Changing the BIAS Level

Normally use 200h for clean sound output. A value of 000h might make sense
during periods when no sound is output (causing the PWM circuit to output
low-bits only, which is eventually reducing the power consumption, and/or
preventing 32KHz noise). Note: Using the SoundBias function (SWI 19h) allows to
change the level by slowly incrementing or decrementing it (without hard
scratch noise).

Low Power Mode

When not using sound output, power consumption can be reduced by setting both
4000084h (PSG/FIFO) and 4000088h (BIAS) to zero.
```
