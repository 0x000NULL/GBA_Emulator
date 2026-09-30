# LCD I/O Window Feature

Source: https://problemkaputt.de/gbatek.htm#lcdiowindowfeature

```
The Window Feature may be used to split the screen into four regions. The
BG0-3,OBJ layers and Color Special Effects can be separately enabled or
disabled in each of these regions.

The DISPCNT Register

DISPCNT Bits 13-15 are used to enable Window 0, Window 1, and/or OBJ Window
regions, if any of these regions is enabled then the "Outside of Windows"
region is automatically enabled, too.

DISPCNT Bits 8-12 are kept used as master enable bits for the BG0-3,OBJ layers,
a layer is displayed only if both DISPCNT and WININ/OUT enable bits are set.

4000040h - WIN0H - Window 0 Horizontal Dimensions (W)

4000042h - WIN1H - Window 1 Horizontal Dimensions (W)

  Bit   Expl.
  0-7   X2, Rightmost coordinate of window, plus 1
  8-15  X1, Leftmost coordinate of window

Garbage values of X2>240 or X1>X2 are interpreted as X2=240.

4000044h - WIN0V - Window 0 Vertical Dimensions (W)

4000046h - WIN1V - Window 1 Vertical Dimensions (W)

  Bit   Expl.
  0-7   Y2, Bottom-most coordinate of window, plus 1
  8-15  Y1, Top-most coordinate of window

Garbage values of Y2>160 or Y1>Y2 are interpreted as Y2=160.

4000048h - WININ - Control of Inside of Window(s) (R/W)

  Bit   Expl.
  0-3   Window 0 BG0-BG3 Enable Bits     (0=No Display, 1=Display)
  4     Window 0 OBJ Enable Bit          (0=No Display, 1=Display)
  5     Window 0 Color Special Effect    (0=Disable, 1=Enable)
  6-7   Not used
  8-11  Window 1 BG0-BG3 Enable Bits     (0=No Display, 1=Display)
  12    Window 1 OBJ Enable Bit          (0=No Display, 1=Display)
  13    Window 1 Color Special Effect    (0=Disable, 1=Enable)
  14-15 Not used

400004Ah - WINOUT - Control of Outside of Windows & Inside of OBJ Window (R/W)

  Bit   Expl.
  0-3   Outside BG0-BG3 Enable Bits      (0=No Display, 1=Display)
  4     Outside OBJ Enable Bit           (0=No Display, 1=Display)
  5     Outside Color Special Effect     (0=Disable, 1=Enable)
  6-7   Not used
  8-11  OBJ Window BG0-BG3 Enable Bits   (0=No Display, 1=Display)
  12    OBJ Window OBJ Enable Bit        (0=No Display, 1=Display)
  13    OBJ Window Color Special Effect  (0=Disable, 1=Enable)
  14-15 Not used

The OBJ Window

The dimension of the OBJ Window is specified by OBJs which are having the "OBJ
Mode" attribute being set to "OBJ Window". Any non-transparent dots of any such
OBJs are marked as OBJ Window area. The OBJ itself is not displayed.

The color, palette, and display priority of these OBJs are ignored. Both
DISPCNT Bits 12 and 15 must be set when defining OBJ Window region(s).

Window Priority

In case that more than one window is enabled, and that these windows do
overlap, Window 0 is having highest priority, Window 1 medium, and Obj Window
lowest priority. Outside of Window is having zero priority, it is used for all
dots which are not inside of any window region.
```
