# BIOS Differences between GBA and NDS functions

Source: https://problemkaputt.de/gbatek.htm#biosdifferencesbetweengbaandndsfunctions

```
Differences between GBA and NDS BIOS functions

- SoftReset uses different addresses

- SWI numbers for Halt, Stop/Sleep, Div, Sqrt have changed

- Halt destroys r0 on NDS9, IntrWait bugged on NDS9

- CpuFastSet allows 4-byte blocks (nice), but...

- CpuFastSet works very SLOW because of a programming bug (uncool)

- Some of the decompression functions are now using callbacks

- SoundBias uses new delay parameter

And, a number of GBA functions have been removed, and some new NDS functions
have been added, see:

BIOS Function Summary
```
