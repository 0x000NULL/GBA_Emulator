# GBA Cheat Codes - Pro Action Replay V3

Source: https://problemkaputt.de/gbatek.htm#gbacheatcodesproactionreplayv3

```
Pro Action Replay V3 - RAW Codes

  C4aaaaaa 0000yyyy  Enable Code - Hook Routine at [8aaaaaa]
  xxxxxxxx 001DC0DE  Enable Code - ID Code [080000AC]
  DEADFACE 0000xxxx  Enable Code - Change Encryption Seeds
  00aaaaaa xxxxxxyy  [a0aaaaa..a0aaaaa+xxxxxx]=yy
  02aaaaaa xxxxyyyy  [a0aaaaa..a0aaaaa+xxxx*2]=yyyy
  04aaaaaa yyyyyyyy  [a0aaaaa]=yyyyyyyy
  40aaaaaa xxxxxxyy  [ [a0aaaaa] + xxxxxx ]=yy   (Indirect)
  42aaaaaa xxxxyyyy  [ [a0aaaaa] + xxxx*2 ]=yyyy (Indirect)
  44aaaaaa yyyyyyyy  [ [a0aaaaa] ]=yyyyyyyy      (Indirect)
  80aaaaaa 000000yy  [a0aaaaa]=[a0aaaaa]+yy
  82aaaaaa 0000yyyy  [a0aaaaa]=[a0aaaaa]+yyyy
  84aaaaaa yyyyyyyy  [a0aaaaa]=[a0aaaaa]+yyyyyyyy
  C6aaaaaa 0000yyyy  [4aaaaaa]=yyyy              (I/O Area)
  C7aaaaaa yyyyyyyy  [4aaaaaa]=yyyyyyyy          (I/O Area)
  iiaaaaaa yyyyyyyy  IF [a0aaaaa] <cond> <value> THEN <action>
  00000000 60000000  ELSE (?)
  00000000 40000000  ENDIF (?)
  00000000 0800xx00  AR Slowdown : loops the AR xx times
  00000000 00000000  End of the code list
  00000000 10aaaaaa 000000zz 00000000  IF AR_BUTTON THEN [a0aaaaa]=zz
  00000000 12aaaaaa 0000zzzz 00000000  IF AR_BUTTON THEN [a0aaaaa]=zzzz
  00000000 14aaaaaa zzzzzzzz 00000000  IF AR_BUTTON THEN [a0aaaaa]=zzzzzzzz
  00000000 18aaaaaa 0000zzzz 00000000  [8000000+aaaaaa*2]=zzzz  (ROM Patch 1)
  00000000 1Aaaaaaa 0000zzzz 00000000  [8000000+aaaaaa*2]=zzzz  (ROM Patch 2)
  00000000 1Caaaaaa 0000zzzz 00000000  [8000000+aaaaaa*2]=zzzz  (ROM Patch 3)
  00000000 1Eaaaaaa 0000zzzz 00000000  [8000000+aaaaaa*2]=zzzz  (ROM Patch 4)

  00000000 80aaaaaa 000000yy ssccssss  repeat cc times [a0aaaaa]=yy
   (with yy=yy+ss, a0aaaaa=a0aaaaa+ssss after each step)

  00000000 82aaaaaa 0000yyyy ssccssss  repeat cc times [a0aaaaa]=yyyy
   (with yyyy=yyyy+ss, a0aaaaa=a0aaaaa+ssss*2 after each step)

  00000000 84aaaaaa yyyyyyyy ssccssss  repeat cc times [a0aaaaa]=yyyyyyyy
   (with yyyy=yyyy+ss, a0aaaaa=a0aaaaa+ssss*4 after each step)

Warning: There is a bug on the real AR (v2 upgraded to v3, and maybe on real
v3) with the 32bit Increment Slide code. You HAVE to add a code (best choice is
80000000 00000000 : add 0 to value at address 0) right after it, else the AR
will erase the 2 last 8 digits lines of the 32 Bits Inc. Slide code when you
enter it !!!

Final Notes

The 'turn off all codes' makes an infinite loop (that can't be broken, unless
the condition becomes True). - How? By Interrupt? Huh?

ROM Patch1 works on real V3 and, on V1/V2 upgraded to V3.

ROM Patch2,3,4 work on real V3 hardware only.

Pro Action Replay V3 Conditional Codes - iiaaaaaa yyyyyyyy

The 'ii' is composed of <cond> + <value> + <action>.

  <cond>           <value>            <action>
  08 Equal =       00 8bit zz         00 execute next code
  10 Not equal <>  02 16bit zzzz      40 execute next two codes
  18 Signed <      04 32bit zzzzzzzz  80 execute all following
  20 Signed >      06 (always false)     codes until ELSE or ENDIF
  28 Unsigned <                       C0 normal ELSE turn off all codes
  30 Unsigned >
  38 Logical AND

For example, ii=18h+02h+40h=5Ah, produces IF [a0aaaaa]<zzzz THEN next 2
codes.

Always... Codes

  For the "Always..." codes:
  - XXXXXXXX can be any authorised address except 00000000 (eg. use 02000000).
  - ZZZZZZZZ can be anything.
  - The "y" in the code data must be in the [1-7] range (which means not 0).
  typ=y,sub=0,siz=3   Always skip next line.
  typ=y,sub=1,siz=3   Always skip next 2 lines.
  typ=y,sub=2,siz=3   Always Stops executing all the codes below.
  typ=y,sub=3,siz=3   Always turn off all codes.

Code Format (ttaaaaaa xxxxyyzz)

 adr mask = 003FFFFF
 n/a mask = 00C00000 ;not used
 xtr mask = 01000000 ;used only by I/O write, and MSB of Hook
 siz mask = 06000000
 typ mask = 38000000 ;0=normal, other=conditional
 sub mask = C0000000

Pro Action Replay V3 Encryption

Works exactly as for Gameshark Encryption, but with different initial seeds,

  S0=7AA9648Fh S1=7FAE6994h S2=C0EFAAD5h S3=42712C57h

And, the T1 and T2 translation tables are different, too.
```
