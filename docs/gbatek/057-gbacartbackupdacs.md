# GBA Cart Backup DACS

Source: https://problemkaputt.de/gbatek.htm#gbacartbackupdacs

```
128 KBytes - 1Mbit DACS - Lifetime: 100,000 writes.

1024 KBytes - 8Mbit DACS - Lifetime: 100,000 writes.

DACS (Debugging And Communication System) is used in Nintendo's hardware
debugger only, DACS is NOT used in normal game cartridges.

Parts of DACS memory is used to store the debugging exception handlers (entry
point/size defined in cartridge header), the remaining memory could be used to
store game positions or other data. The address space is the upper end of the
32MB ROM area, the memory can be read directly by the CPU, including for
ability to execute program code in this area.
```
