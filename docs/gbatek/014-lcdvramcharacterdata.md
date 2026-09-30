# LCD VRAM Character Data

Source: https://problemkaputt.de/gbatek.htm#lcdvramcharacterdata

```
Each character (tile) consists of 8x8 dots (64 dots in total). The color depth
may be either 4bit or 8bit (see BG0CNT-BG3CNT).

4bit depth (16 colors, 16 palettes)

Each tile occupies 32 bytes of memory, the first 4 bytes for the topmost row of
the tile, and so on. Each byte representing two dots, the lower 4 bits define
the color for the left (!) dot, the upper 4 bits the color for the right dot.

8bit depth (256 colors, 1 palette)

Each tile occupies 64 bytes of memory, the first 8 bytes for the topmost row of
the tile, and so on. Each byte selects the palette entry for each dot.
```
