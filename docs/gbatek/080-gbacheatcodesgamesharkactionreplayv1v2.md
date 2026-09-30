# GBA Cheat Codes - Gameshark/Action Replay V1/V2

Source: https://problemkaputt.de/gbatek.htm#gbacheatcodesgamesharkactionreplayv1v2

```
Gameshark RAW Codes (These codes must be encrypted before using them)

  0aaaaaaa 000000xx  [aaaaaaa]=xx
  1aaaaaaa 0000xxxx  [aaaaaaa]=xxxx
  2aaaaaaa xxxxxxxx  [aaaaaaa]=xxxxxxxx
  3000cccc xxxxxxxx  write xxxxxxxx to (cccc-1) addresses (list in next codes)
  aaaaaaaa aaaaaaaa  parameter for above code, containing two addresses each
  aaaaaaaa 00000000  last parameter for above, zero-padded if only one address
  60aaaaaa y000xxxx  [8000000h+aaaaaa*2]=xxxx (ROM Patch)
  8a1aaaaa 000000xx  IF GS_Button_Down THEN [a0aaaaa]=xx
  8a2aaaaa 0000xxxx  IF GS_Button_Down THEN [a0aaaaa]=xxxx
  80F00000 0000xxxx  IF GS_Button_Down THEN slowdown xxxx * ? cycles per hook
  Daaaaaaa 0000xxxx  IF [aaaaaaa]=xxxx THEN (next code)
  E0zzxxxx 0aaaaaaa  IF [aaaaaaa]=xxxx THEN (next 'zz' codes)
  Faaaaaaa 00000x0y  Enable Code - Hook Routine
  xxxxxxxx 001DC0DE  Enable Code - Game Code ID (value at [0ACh] in cartridge)
  DEADFACE 0000xxyy  Change Encryption Seeds

Enable Code - Hook Routine

Hook Address 'aaaaaaa' is a 28bit ROM address (8FFFFFFh-9FFFFFFh).

Used to insert the GS code handler routine where it will be executed at

least 20 times per second. Without this code, GSA can not write to RAM.

 y=1 - Executes code handler without backing up the LR register.
 y=2 - Executes code handler and backs up the LR register.
 y=3 - Replaces a 32-bit pointer used for long-branches.
 x=0 - Must turn GSA off before loading game.
 x=1 - Must not do that.

ROM Patch

This type allows GSA to intercept ROM reads and returns the value xxxx.

 y=0 wait for the code handler to enable the patch
 y=1 patch is enabled before the game starts
 y=2 unknown ?

Note: V1/V2 hardware can only have up to 1 user-defined rom patch max. V3 can
have up to 4. Some enable code types can shorten the amount of user-defined rom
patches available.

Gameshark Encryption

A=Left half, and V=Right half of code.

  FOR I=1 TO 32
    A=A + (V*16+S0) XOR (V+I*9E3779B9h) XOR (V/32+S1)
    V=V + (A*16+S2) XOR (A+I*9E3779B9h) XOR (A/32+S3)
  NEXT I

Upon startup, the initial encryption seeds are:

  S0=09F4FBBDh S1=9681884Ah S2=352027E9h S3=F3DEE5A7h

Upon DEADFACE 0000xxyy, the S0..S3 seeds are changed like so:

  FOR y=0 TO 3
   FOR x=0 TO 3
    z = T1[(xx+x) AND FFh] + T2[(yy+y) AND FFh]
    Sy = Sy*100h + (z AND FFh)
   NEXT x
  NEXT y

All calculations truncated to unsigned 32bit integer values.

T1 and T2 are translation tables contained in the gameshark cartridge.
```
