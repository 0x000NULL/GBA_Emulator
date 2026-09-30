# GBA Cart e-Reader File Formats

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderfileformats

```
.BMP Files (homebrew 300 DPI strips)

Contains a picture of the whole dotcode strip with address bars and sync marks
(see Dotcode chapter) in Microsoft's Bitmap format. The image is conventionally
surrounded by a blank 2-pixel border, resulting in a size of 989x44 pixels for
long strips. The file should should have 1bit color depth. The pixels per meter
entry should match the desired printing resolution, either 300 DPI or 360 DPI.
But, resolution of printer hardware is typically specified in inch rather than
in meters, so an exact match isn't supported by Microsoft. Most homebrew .BMP
files contain nonsense resolutions like 200 DPI, or 300 dots per meter (ca. 8
DPI).

.JPG Files (scanned 1200 DPI strips)

Same as BMP, but should contain a dotcode scanned at 1200 DPI, with correct
orientation (the card-edge side at the bottom of the image), and containing
only the dotcode (not the whole card), so the JPG size should be about 3450x155
pixels for long strips.

No$gba currently doesn't work with progressive JPGs. Scans with white
background can be saved as monochrome JPG. Scans with red/yellow background
should contain a correct RED layer (due to the red LED light source) (the
brightness of the green/blue layers can be set to zero for better compression).

.RAW Files

Contains the "raw" information from the BMP format, that is, 2-byte block
header, 102-byte data, 2-byte block header, 102-byte data, etc. The data
portion is interleaved, and includes the full 48-byte data header, titles, vpk
compressed data, error-info, and unused bytes. RAW files are excluding Address
Bars, Sync Marks, and 4bit-to-5bit encoding.

Each RAW file contains one or more strip(s), so the RAW filesize is either
18*104 bytes (short strip), or 28*104 bytes (long strip), or a multiple thereof
(if it contains more than one strip) (although multi-strip games are often
stored in separate files for each strip; named file1.raw, file2.raw, etc).

.BIN Files

Filesize should be I*30h, with I=1Ch for short strips, and I=2Ch for long
strips, or a multiple thereof (if it contains more than one strip). Each strip
consists of the 48-byte Data Header, followed by title(s), and vpk compressed
data. Unlike .RAW files, .BIN files aren't interleaved, and do not contain
Block Headers, nor error-info, nor unused bytes (in last block). The files do
contain padding bytes to match a full strip-size of I*30h.

Caution: Older .BIN files have been using a size-reduced 12-byte header (taken
from entries 0Dh, 0Ch, 10h-11h, 26h-2Dh of the 48-byte Data Header; in that
order), that files have never contained more than one strip per file, so the
filesize should be exactly I*30h-36, the size-reduced header doesn't contain a
Primary Type entry, so it's everyone's bet which Card Type is to be used (hint:
the 12-byte headers were based on the assumption that Primary Type would be
always 01h on Short Strips, and 02h on Long Strips).

.SAV Files

Contains a copy of the e-Reader's 128Kbyte FLASH memory. With the saved
e-Reader application being located in the 2nd 64K-bank, the data consists of a
header with title and gba/nes/z80 format info, followed by the vpk compressed
data. The FLASH memory does also contain e-Reader calibration settings, the
remaining 100Kbytes are typically FFh-filled.
```
