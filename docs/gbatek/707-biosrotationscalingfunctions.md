# BIOS Rotation/Scaling Functions

Source: https://problemkaputt.de/gbatek.htm#biosrotationscalingfunctions

```
BgAffineSet

ObjAffineSet

SWI 0Eh (GBA) - BgAffineSet

Used to calculate BG Rotation/Scaling parameters.

  r0   Pointer to Source Data Field with entries as follows:
        s32  Original data's center X coordinate (8bit fractional portion)
        s32  Original data's center Y coordinate (8bit fractional portion)
        s16  Display's center X coordinate
        s16  Display's center Y coordinate
        s16  Scaling ratio in X direction (8bit fractional portion)
        s16  Scaling ratio in Y direction (8bit fractional portion)
        u16  Angle of rotation (8bit fractional portion) Effective Range 0-FFFF
  r1   Pointer to Destination Data Field with entries as follows:
        s16  Difference in X coordinate along same line
        s16  Difference in X coordinate along next line
        s16  Difference in Y coordinate along same line
        s16  Difference in Y coordinate along next line
        s32  Start X coordinate
        s32  Start Y coordinate
  r2   Number of Calculations

Return: No return value, Data written to destination address.

SWI 0Fh (GBA) - ObjAffineSet

Calculates and sets the OBJ's affine parameters from the scaling ratio and
angle of rotation.

The affine parameters are calculated from the parameters set in Srcp.

The four affine parameters are set every Offset bytes, starting from the Destp
address.

If the Offset value is 2, the parameters are stored contiguously. If the value
is 8, they match the structure of OAM.

When Srcp is arrayed, the calculation can be performed continuously by
specifying Num.

  r0   Source Address, pointing to data structure as such:
        s16  Scaling ratio in X direction (8bit fractional portion)
        s16  Scaling ratio in Y direction (8bit fractional portion)
        u16  Angle of rotation (8bit fractional portion) Effective Range 0-FFFF
  r1   Destination Address, pointing to data structure as such:
        s16  Difference in X coordinate along same line
        s16  Difference in X coordinate along next line
        s16  Difference in Y coordinate along same line
        s16  Difference in Y coordinate along next line
  r2   Number of calculations
  r3   Offset in bytes for parameter addresses (2=continuous, 8=OAM)

Return: No return value, Data written to destination address.

For both Bg- and ObjAffineSet, Rotation angles are specified as 0-FFFFh
(covering a range of 360 degrees), however, the GBA BIOS recurses only the
upper 8bit; the lower 8bit may contain a fractional portion, but it is ignored
by the BIOS.
```
