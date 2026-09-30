# GBA Cart e-Reader Error Correction

Source: https://problemkaputt.de/gbatek.htm#gbacartereadererrorcorrection

```
The Error Correction Information that is appended at the end of the Block
Header & Data Fragments consists of standard Reed-Solomon codes, which are
also used for CD/DVD disks, DSL modems, and digital DVB television signals.
That info allows to locate and repair a number of invalid data bytes.

Below code shows how to create and verify error-info (but not how to do the
actual error correction). The dtalen,errlen values should be 18h,10h for the
Block Header, and 40h,10h for Data Fragments; the latter settings might be
possible to get changed to other values though?

append_error_info(data,dtalen,errlen)

  reverse_byte_order(data,dtalen)
  zerofill_error_bytes(data,errlen)
  for i=dtalen-1 to errlen  ;loop across data portion
    z = rev[ data[i] xor data[errlen-1] ] ;
    for j=errlen-1 to 0     ;loop across error-info portion
    if j=0 then x=00h else x=data[j-1]
      if z<>FFh then
        y=gg[j], if y<>FFh then
          y=y+z, if y>=FFh then y=y-FFh
          x=x xor pow[y]
      data[j]=x
    next j
  next i
  invert_error_bytes(data,errlen)
  reverse_byte_order(data,dtalen)

verify_error_info(data,dtalen,errlen)

  reverse_byte_order(data,dtalen)
  invert_error_bytes(data,errlen)
  make_rev(data,dtalen)
  for i=78h to 78h+errlen-1
    x=0, z=0
    for j=0 to dtalen-1
      y=data[j]
      if y<>FFh then
        y=y+z, if y>=FFh then y=y-FFh
        x=x xor pow[y]
      z=z+i, if z>=FFh then z=z-FFh
    next j
    if x<>0 then error
  next i
  ;(if errors occured, could correct them now)
  make_pow(data,dtalen)
  invert_error_bytes(data,errlen)
  reverse_byte_order(data,dtalen)

make_rev(data,len)

  for i=0 to len-1, data[i]=rev[data[i]], next i

make_pow(data,len)

  for i=0 to len-1, data[i]=pow[data[i]], next i

invert_error_bytes(data,len)

  for i=0 to len-1, data[i]=data[i] xor FFh, next i

zerofill_error_bytes(data,len)

  for i=0 to len-1, data[i]=00h, next i

reverse_byte_order(data,len)

  for i=0 to (len-1)/2, x=data[i], data[i]=data[len-i], data[len-i]=x, next i

create_pow_and_rev_tables

  x=01h, pow[FFh]=00h, rev[00h]=FFh
  for i=00h to FEh
    pow[i]=x, rev[x]=i, x=x*2, if x>=100h then x=x xor 187h
  next i

create_gg_table

  gg[0]=pow[78h]
  for i=1 to errlen-1
    gg[i]=01h
    for j=i downto 0
      if j=0 then y=00h else y=gg[j-1]
      x=gg[j], if x<>00h then
        x=rev[x]+78h+i, if x>=FFh then x=x-FFh
        y=y xor pow[x]
      gg[j]=y
    next j
  next i
  make_rev(gg,errlen)

With above value of 78h, and errlen=10h, gg[00h..0Fh] will be always:

  00h,4Bh,EBh,D5h,EFh,4Ch,71h,00h,F4h,00h,71h,4Ch,EFh,D5h,EBh,4Bh

So using a hardcoded table should take up less memory than calculating it.

Notes

The actual error correction should be able to fix up to "errlen" errors at
known locations (eg. data from blocks that haven't been scanned, or whose
5bit-to-4bit conversion had failed due to an invalid 5bit value), or up to
"errlen/2" errors at unknown locations. The corrected data isn't guaranteed to
be correct (even if it looks okay to the "verify" function), so the Data Header
checksums should be checked, too.

More Info

For more info, I've found Reed-Solomon source code from Simon Rockliff, and an
updated version from Robert Morelos-Zaragoza and Hari Thirumoorthy to be
useful. For getting started with that source, some important relationships
& differences are:

  pow = alpha_to, but generated as shown above
  rev = index_of, dito
  b0  = 78h
  nn  = dtalen
  kk  = dtalen-errlen
  %nn = MOD FFh (for the ereader that isn't MOD dtalen)
  -1  = FFh

And, the ereader processes data/errinfo backwards, starting at the last byte.
```
