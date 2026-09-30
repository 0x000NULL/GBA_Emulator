# GBA Sound Channel 4 - Noise

Source: https://problemkaputt.de/gbatek.htm#gbasoundchannel4noise

```
This channel is used to output white noise. This is done by randomly switching
the amplitude between high and low at a given frequency. Depending on the
frequency the noise will appear 'harder' or 'softer'.

It is also possible to influence the function of the random generator, so the
that the output becomes more regular, resulting in a limited ability to output
Tone instead of Noise.

4000078h - SOUND4CNT_L (NR41, NR42) - Channel 4 Length/Envelope (R/W)

  Bit        Expl.
  0-5   W    Sound length; units of (64-n)/256s  (0-63)
  6-7   -    Not used
  8-10  R/W  Envelope Step-Time; units of n/64s  (1-7, 0=No Envelope)
  11    R/W  Envelope Direction                  (0=Decrease, 1=Increase)
  12-15 R/W  Initial Volume of envelope          (1-15, 0=No Sound)
  16-31 -    Not used

The Length value is used only if Bit 6 in NR44 is set.

400007Ch - SOUND4CNT_H (NR43, NR44) - Channel 4 Frequency/Control (R/W)

The amplitude is randomly switched between high and low at the given frequency.
A higher frequency will make the noise to appear 'softer'.

When Bit 3 is set, the output will become more regular, and some frequencies
will sound more like Tone than Noise.

  Bit        Expl.
  0-2   R/W  Dividing Ratio of Frequencies (r)
  3     R/W  Counter Step/Width (0=15 bits, 1=7 bits)
  4-7   R/W  Shift Clock Frequency (s)
  8-13  -    Not used
  14    R/W  Length Flag  (1=Stop output when length in NR41 expires)
  15    W    Initial      (1=Restart Sound)
  16-31 -    Not used

Frequency = 524288 Hz / r / 2^(s+1)     ;For r=0 assume r=0.5 instead

Noise Random Generator (aka Polynomial Counter)

Noise randomly switches between HIGH and LOW levels, the output levels are
calculated by a shift register (X), at the selected frequency, as such:

  7bit:  X=X SHR 1, IF carry THEN Out=HIGH, X=X XOR 60h ELSE Out=LOW
  15bit: X=X SHR 1, IF carry THEN Out=HIGH, X=X XOR 6000h ELSE Out=LOW

The initial value when (re-)starting the sound is X=40h (7bit) or X=4000h
(15bit). The data stream repeats after 7Fh (7bit) or 7FFFh (15bit) steps.
```
