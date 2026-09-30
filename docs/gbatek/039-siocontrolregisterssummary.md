# SIO Control Registers Summary

Source: https://problemkaputt.de/gbatek.htm#siocontrolregisterssummary

```
Mode Selection (by RCNT.15-14 and SIOCNT.13-12)

  R.15 R.14 S.13 S.12 Mode
  0    x    0    0    Normal 8bit
  0    x    0    1    Normal 32bit
  0    x    1    0    Multiplay 16bit
  0    x    1    1    UART (RS232)
  1    0    x    x    General Purpose
  1    1    x    x    JOY BUS

SIOCNT

  Bit    0      1    2     3      4 5 6   7     8    9      10   11
  Normal Master Rate SI/In SO/Out - - -   Start -    -      -    -
  Multi  Baud   Baud SI/In SD/In  ID# Err Start -    -      -    -
  UART   Baud   Baud CTS   Parity S R Err Bits  FIFO Parity Send Recv
```
