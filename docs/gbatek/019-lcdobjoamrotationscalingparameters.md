# LCD OBJ - OAM Rotation/Scaling Parameters

Source: https://problemkaputt.de/gbatek.htm#lcdobjoamrotationscalingparameters

```
As described in the previous chapter, there are blank spaces between each of
the 128 OBJ Attribute Fields in OAM memory. These 128 16bit gaps are used to
store OBJ Rotation/Scaling Parameters.

Location of Rotation/Scaling Parameters in OAM

Four 16bit parameters (PA,PB,PC,PD) are required to define a complete group of
Rotation/Scaling data. These are spread across OAM as such:

  1st Group - PA=07000006, PB=0700000E, PC=07000016, PD=0700001E
  2nd Group - PA=07000026, PB=0700002E, PC=07000036, PD=0700003E
  etc.

By using all blank space (128 x 16bit), up to 32 of these groups (4 x 16bit
each) can be defined in OAM.

OBJ Rotation/Scaling PA,PB,PC,PD Parameters (R/W)

Each OBJ that uses Rotation/Scaling may select between any of the above 32
parameter groups. For details, refer to the previous chapter about OBJ
Attributes.

The meaning of the separate PA,PB,PC,PD values is identical as for BG, for
details read the chapter about BG Rotation/Scaling.

OBJ Reference Point & Rotation Center

The OBJ Reference Point is the upper left of the OBJ, ie. OBJ X/Y coordinates:
X+0, Y+0.

The OBJ Rotation Center is always (or should be usually?) in the middle of the
object, ie. for a 8x32 pixel OBJ, this would be at the OBJ X/Y coordinates:
X+4, and Y+16.

OBJ Double-Size Bit (for OBJs that use Rotation/Scaling)

When Double-Size is zero: The sprite is rotated, and then display inside of the
normal-sized (not rotated) rectangular area - the edges of the rotated sprite
will become invisible if they reach outside of that area.

When Double-Size is set: The sprite is rotated, and then display inside of the
double-sized (not rotated) rectangular area - this ensures that the edges of
the rotated sprite remain visible even if they would reach outside of the
normal-sized area. (Except that, for example, rotating a 8x32 pixel sprite by
90 degrees would still cut off parts of the sprite as the double-size area
isn't large enough.)
```
