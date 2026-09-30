# BIOS Sound Functions

Source: https://problemkaputt.de/gbatek.htm#biossoundfunctions

```
MidiKey2Freq

SoundBias

SoundChannelClear

SoundDriverInit

SoundDriverMain

SoundDriverMode

SoundDriverVSync

SoundDriverVSyncOff

SoundDriverVSyncOn

SoundWhatever0..4

SoundGetJumpList

SWI 1Fh (GBA) - MidiKey2Freq

Calculates the value of the assignment to ((SoundArea)sa).vchn[x].fr when
playing the wave data, wa, with the interval (MIDI KEY) mk and the fine
adjustment value (halftones=256) fp.

  r0  WaveData* wa
  r1  u8 mk
  r2  u8 fp

Return:

  r0  u32

This function is particularly popular because it allows to read from BIOS
memory without copy protection range checks. The formula to read one byte (a)
from address (i, 0..3FFF) is:

a = (MidiKey2Freq(i-(((i AND 3)+1)OR 3), 168, 0) * 2) SHR 24

SWI 19h (GBA) or SWI 08h (NDS7/DSi7) - SoundBias

Increments or decrements the current level of the SOUNDBIAS register (with
short delays) until reaching the desired new level. The upper bits of the
register are kept unchanged.

  r0   BIAS level (0=Level 000h, any other value=Level 200h)
  r1   Delay Count (NDS/DSi only) (GBA uses a fixed delay count of 8)

Return: No return value.

SWI 1Eh (GBA) - SoundChannelClear

Clears all direct sound channels and stops the sound.

This function may not operate properly when the library which expands the sound
driver feature is combined afterwards. In this case, do not use it.

No parameters, no return value.

SWI 1Ah (GBA) - SoundDriverInit

Initializes the sound driver. Call this only once when the game starts up.

It is essential that the work area already be secured at the time this function
is called.

You cannot execute this driver multiple times, even if separate work areas have
been prepared.

  r0  Pointer to work area for sound driver, SoundArea structure as follows:
       SoundArea (sa) Structure
        u32    ident      Flag the system checks to see whether the
                          work area has been initialized and whether it
                          is currently being accessed.
        vu8    DmaCount   User access prohibited
        u8     reverb     Variable for applying reverb effects to direct sound
        u16    d1         User access prohibited
        void   (*func)()  User access prohibited
        int    intp       User access prohibited
        void*  NoUse      User access prohibited
        SndCh  vchn[MAX]  The structure array for controlling the direct
                          sound channels (currently 8 channels are
                          available). The term "channel" here does
                          not refer to hardware channels, but rather to
                          virtual constructs inside the sound driver.
        s8     pcmbuf[PCM_BF*2]
       SoundChannel Structure
        u8         sf     The flag indicating the status of this channel.
                          When 0 sound is stopped.
                          To start sound, set other parameters and
                          then write 80h to here.
                          To stop sound, logical OR 40h for a
                          release-attached off (key-off), or write zero
                          for a pause. The use of other bits is
                          prohibited.
        u8         r1     User access prohibited
        u8         rv     Sound volume output to right side
        u8         lv     Sound volume output to left side
        u8         at     The attack value of the envelope. When the
                          sound starts, the volume begins at zero and
                          increases every 1/60 second. When it
                          reaches 255, the process moves on to the
                          next decay value.
        u8         de     The decay value of the envelope. It is
                          multiplied by "this value/256" every 1/60
                          sec. and when sustain value is reached, the
                          process moves to the sustain condition.
        u8         su     The sustain value of the envelope. The
                          sound is sustained by this amount.
                          (Actually, multiplied by rv/256, lv/256 and
                          output left and right.)
        u8         re     The release value of the envelope. Key-off
                          (logical OR 40h in sf) to enter this state.
                          The value is multiplied by "this value/256"
                          every 1/60 sec. and when it reaches zero,
                          this channel is completely stopped.
        u8         r2[4]  User access prohibited
        u32        fr     The frequency of the produced sound.
                          Write the value obtained with the
                          MidiKey2Freq function here.
        WaveData*  wp     Pointer to the sound's waveform data. The waveform
                          data can be generated automatically from the AIFF
                          file using the tool (aif2agb.exe), so users normally
                          do not need to create this themselves.
        u32        r3[6]  User access prohibited
        u8         r4[4]  User access prohibited
       WaveData Structure
        u16   type    Indicates the data type. This is currently not used.
        u16   stat    At the present time, non-looped (1 shot) waveform
                      is 0000h and forward loop is 4000h.
        u32   freq    This value is used to calculate the frequency.
                      It is obtained using the following formula:
                      sampling rate x 2^((180-original MIDI key)/12)
        u32   loop    Loop pointer (start of loop)
        u32   size    Number of samples (end position)
        s8    data[]  The actual waveform data. Takes (number of samples+1)
                      bytes of 8bit signed linear uncompressed data. The last
                      byte is zero for a non-looped waveform, and the same
                      value as the loop pointer data for a looped waveform.

Return: No return value.

SWI 1Ch (GBA) - SoundDriverMain

Main of the sound driver.

Call every 1/60 of a second. The flow of the process is to call
SoundDriverVSync, which is explained later, immediately after the V-Blank
interrupt.

After that, this routine is called after BG and OBJ processing is executed.

No parameters, no return value.

SWI 1Bh (GBA) - SoundDriverMode

Sets the sound driver operation mode.

  r0  Sound driver operation mode
       Bit    Expl.
       0-6    Direct Sound Reverb value (0-127, default=0) (ignored if Bit7=0)
       7      Direct Sound Reverb set (0=ignore, 1=apply reverb value)
       8-11   Direct Sound Simultaneously-produced (1-12 channels, default 8)
       12-15  Direct Sound Master volume (1-15, default 15)
       16-19  Direct Sound Playback Frequency (1-12 = 5734,7884,10512,13379,
              15768,18157,21024,26758,31536,36314,40137,42048, def 4=13379 Hz)
       20-23  Final number of D/A converter bits (8-11 = 9-6bits, def. 9=8bits)
       24-31  Not used.

Return: No return value.

SWI 1Dh (GBA) - SoundDriverVSync

An extremely short system call that resets the sound DMA. The timing is
extremely critical, so call this function immediately after the V-Blank
interrupt every 1/60 second.

No parameters, no return value.

SWI 28h (GBA) - SoundDriverVSyncOff

Due to problems with the main program if the V-Blank interrupts are stopped,
and SoundDriverVSync cannot be called every 1/60 a second, this function must
be used to stop sound DMA.

Otherwise, even if you exceed the limit of the buffer the DMA will not stop and
noise will result.

No parameters, no return value.

SWI 29h (GBA) - SoundDriverVSyncOn

This function restarts the sound DMA stopped with the previously described
SoundDriverVSyncOff.

After calling this function, have a V-Blank occur within 2/60 of a second and
call SoundDriverVSync.

No parameters, no return value.

SWI 20h..24h (GBA) - SoundWhatever0..4 (Undocumented)

Whatever undocumented sound-related BIOS functions.

SWI 2Ah (GBA) - SoundGetJumpList (Undocumented)

Receives pointers to 36 additional sound-related BIOS functions.

  r0  Destination address (must be aligned by 4) (120h bytes buffer)
```
