# GBA Sound Channel 3 - Wave Output

Source: https://problemkaputt.de/gbatek.htm#gbasoundchannel3waveoutput

```
This channel can be used to output digital sound, the length of the sample
buffer (Wave RAM) can be either 32 or 64 digits (4bit samples). This sound
channel can be also used to output normal tones when initializing the Wave RAM
by a square wave. This channel doesn't have a volume envelope register.

4000070h - SOUND3CNT_L (NR30) - Channel 3 Stop/Wave RAM select (R/W)

  Bit        Expl.
  0-4   -    Not used
  5     R/W  Wave RAM Dimension   (0=One bank/32 digits, 1=Two banks/64 digits)
  6     R/W  Wave RAM Bank Number (0-1, see below)
  7     R/W  Sound Channel 3 Off  (0=Stop, 1=Playback)
  8-15  -    Not used

The currently selected Bank Number (Bit 6) will be played back, while
reading/writing to/from wave RAM will address the other (not selected) bank.
When dimension is set to two banks, output will start by replaying the
currently selected bank.

4000072h - SOUND3CNT_H (NR31, NR32) - Channel 3 Length/Volume (R/W)

  Bit        Expl.
  0-7   W    Sound length; units of (256-n)/256s  (0-255)
  8-12  -    Not used.
  13-14 R/W  Sound Volume  (0=Mute/Zero, 1=100%, 2=50%, 3=25%)
  15    R/W  Force Volume  (0=Use above, 1=Force 75% regardless of above)

The Length value is used only if Bit 6 in NR34 is set.

4000074h - SOUND3CNT_X (NR33, NR34) - Channel 3 Frequency/Control (R/W)

  Bit        Expl.
  0-10  W    Sample Rate; 2097152/(2048-n) Hz   (0-2047)
  11-13 -    Not used
  14    R/W  Length Flag  (1=Stop output when length in NR31 expires)
  15    W    Initial      (1=Restart Sound)
  16-31 -    Not used

The above sample rate specifies the number of wave RAM digits per second, the
actual tone frequency depends on the wave RAM content, for example:

  Wave RAM, single bank 32 digits   Tone Frequency
  FFFFFFFFFFFFFFFF0000000000000000  65536/(2048-n) Hz
  FFFFFFFF00000000FFFFFFFF00000000  131072/(2048-n) Hz
  FFFF0000FFFF0000FFFF0000FFFF0000  262144/(2048-n) Hz
  FF00FF00FF00FF00FF00FF00FF00FF00  524288/(2048-n) Hz
  F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0  1048576/(2048-n) Hz

4000090h - WAVE_RAM0_L - Channel 3 Wave Pattern RAM (W/R)

4000092h - WAVE_RAM0_H - Channel 3 Wave Pattern RAM (W/R)

4000094h - WAVE_RAM1_L - Channel 3 Wave Pattern RAM (W/R)

4000096h - WAVE_RAM1_H - Channel 3 Wave Pattern RAM (W/R)

4000098h - WAVE_RAM2_L - Channel 3 Wave Pattern RAM (W/R)

400009Ah - WAVE_RAM2_H - Channel 3 Wave Pattern RAM (W/R)

400009Ch - WAVE_RAM3_L - Channel 3 Wave Pattern RAM (W/R)

400009Eh - WAVE_RAM3_H - Channel 3 Wave Pattern RAM (W/R)

This area contains 16 bytes (32 x 4bits) Wave Pattern data which is output by
channel 3. Data is played back ordered as follows: MSBs of 1st byte, followed
by LSBs of 1st byte, followed by MSBs of 2nd byte, and so on - this results in
a confusing ordering when filling Wave RAM in units of 16bit data - ie. samples
would be then located in Bits 4-7, 0-3, 12-15, 8-11.

In the GBA, two Wave Patterns exists (each 32 x 4bits), either one may be
played (as selected in NR30 register), the other bank may be accessed by the
users. After all 32 samples have been played, output of the same bank (or other
bank, as specified in NR30) will be automatically restarted.

Internally, Wave RAM is a giant shift-register, there is no pointer which is
addressing the currently played digit. Instead, the entire 128 bits are
shifted, and the 4 least significant bits are output.

Thus, when reading from Wave RAM, data might have changed its position. And,
when writing to Wave RAM all data should be updated (it'd be no good idea to
assume that old data is still located at the same position where it has been
written to previously).
```
