# LCD OBJ - VRAM Character (Tile) Mapping

Source: https://problemkaputt.de/gbatek.htm#lcdobjvramcharactertilemapping

```
Each OBJ tile consists of 8x8 dots, however, bigger OBJs can be displayed by
combining several 8x8 tiles. The horizontal and vertical size for each OBJ may
be separately defined in OAM, possible H/V sizes are 8,16,32,64 dots - allowing
'square' OBJs to be used (such like 8x8, 16x16, etc) as well as 'rectangular'
OBJs (such like 8x32, 64x16, etc.)

When displaying an OBJ that contains of more than one 8x8 tile, one of the
following two mapping modes can be used. In either case, the tile number of the
upperleft tile must be specified in OAM memory.

Two Dimensional Character Mapping (DISPCNT Bit 6 cleared)

This mapping mode assumes that the 1024 OBJ tiles are arranged as a matrix of
32x32 tiles / 256x256 pixels (In 256 color mode: 16x32 tiles / 128x256 pixels).
Ie. the upper row of this matrix contains tiles 00h-1Fh, the next row tiles
20h-3Fh, and so on.

For example, when displaying a 16x16 pixel OBJ, with tile number set to 04h;
The upper row of the OBJ will consist of tile 04h and 05h, the next row of 24h
and 25h. (In 256 color mode: 04h and 06h, 24h and 26h.)

One Dimensional Character Mapping (DISPCNT Bit 6 set)

In this mode, tiles are mapped each after each other from 00h-3FFh.

Using the same example as above, the upper row of the OBJ will consist of tile
04h and 05h, the next row of tile 06h and 07h. (In 256 color mode: 04h and 06h,
08h and 0Ah.)
```
