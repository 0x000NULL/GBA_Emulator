# GBA Cart e-Reader Data Format

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderdataformat

```
Data Strip Format

The size of the data region is I*48 bytes (I=Interleave Value, see Dotcode
Format), the first 48-byte fragment contains the Data Header, the remaining
(I-1) fragments are Data Fragments (which contain title(s), and VPK compressed
program code).

First Strip

  Data Header  (48 bytes)
  Main-Title   (17 bytes, or 33 bytes)
  Sub-Title(s) (3+18 bytes, or 33 bytes) (for each strip) (optional)
  VPK Size     (2 byte value, total length of VPK Data in ALL strips)
  NULL Value   (4 bytes, contained ONLY in 1st strip of GBA strips)
  VPK Data     (length as defined in VPK Size entry, see above)

Further Strip(s)

  Data Header  (48 bytes)
  Main-Title   (17 bytes, or 33 bytes)
  Sub-Title(s) (3+18 bytes, or 33 bytes) (for each strip) (optional)
  VPK Data     (continued from previous strip)

Data Header (30h bytes) (1st fragment)

  00h-01h  Fixed         (00h,30h)
  02h      Fixed         (01h)     ;01h="Do not calculate Global Checksum" ?
  03h      Primary Type  (see below)
  04h-05h  Fixed         (00h,01h) (don't care)
  06h-07h  Strip Size    (0510h=Short, 0810h=Long Strip) ((I-1)*30h) (MSB,LSB)
  08h-0Bh  Fixed         (00h,00h,10h,12h)
  0Ch-0Dh  Region/Type   (see below)
  0Eh      Strip Type    (02h=Short Strip, 01h=Long Strip) (don't care)
  0Fh      Fixed         (00h) (don't care)
  10h-11h  Unknown       (whatever) (don't care)
  12h      Fixed         (10h)     ;10h="Do calculate Data Checksum" ?
  13h-14h  Data Checksum (see below) (MSB,LSB)
  15h-19h  Fixed         (19h,00h,00h,00h,08h)
  1Ah-21h  ID String     ('NINTENDO')
  22h-25h  Fixed         (00h,22h,00h,09h)
  26h-29h  Size Info     (see below)
  2Ah-2Dh  Flags         (see below)
  2Eh      Header Checksum (entries [0Ch-0Dh,10h-11h,26h-2Dh] XORed together)
  2Fh      Global Checksum (see below)

Primary Type [03h] is 8bit,

  0      Card Type (upper bit) (see below)
  1      Unknown (usually opposite of Bit0) (don't care)
  2-7    Unknown (usually zero)

Region/Type [0Ch..0Dh] is 16bit,

  0-3    Unknown (don't care)
  4-7    Card Type (lower bits) (see below)
  8-11   Region/Version (0=Japan/Original, 1=Non-japan, 2=Japan/Plus)
  12-15  Unknown (don't care)

Size Info [26h-29h] is 32bit,

  0      Unknown            (don't care)
  1-4    Strip Number       (01h..Number of strips)
  5-8    Number of Strips   (01h..0Ch) (01h..08h for Japan/Original version)
  9-23   Size of all Strips (excluding Headers and Main/Sub-Titles)
         (same as "VPK Size", but also including the 2-byte "VPK Size" value,
         plus the 4-byte NULL value; if it is present)
  24-31  Fixed              (02h) (don't care)

Flags [2Ah-2Dh] is 32bit,

  0      Permission to save (0=Start Immediately, 1=Prompt for FLASH Saving)
  1      Sub-Title Flag     (0=Yes, 1=None)    (Japan/Original: always 0=Yes)
  2      Application Type   (0=GBA/Z80, 1=NES) (Japan/Original: always 0=Z80)
  3-31   Zero (0) (don't care)

Data Checksum [13h-14h] is the complement (NOT) of the sum of all halfwords in
all Data Fragments, however, it's all done in reversed byte order: checksum is
calculated with halfwords that are read in MSB,LSB order, and the resulting
checksum is stored in MSB,LSB order in the Header Fragment.

Global Checksum [2Fh] is the complement (NOT) of the sum of the first 2Fh bytes
in the Data Header plus the sum of all Data Fragment checksums; the Data
Fragment checksums are all 30h bytes in a fragment XORed with each other.

Titles (3+N bytes, or N bytes)

Titles can be 33 bytes for both Main and Sub (Format 0Eh), or Main=17 bytes and
Sub=3+18 bytes (Formats 02h..05h). In the 3+N bytes form, the first 3 bytes
(24bit) are are used to display "stats" information in form of "HP: h1 ID:
i1-i2-i3", defined as:

  Bit    Expl.
  0-3    h1, values 1..15 shown as "10..150", value 0 is not displayed
  4-6    i3, values 0..7 shown as "A..G,#"
  7-13   i2, values 0..98 shown as "01..99" values 99..127 as "A0..C8"
  14-18  i1, values 0..31 shown as "A..Z,-,_,{HP},.,{ID?},:"
  19-22  Unknown
  23     Disable stats (0=Show as "HP: h1 ID: i1-i2-i3", 1=Don't show it)

The N bytes portion contains the actual title, which must be terminated by 00h
(so the max length is N-1 characters, if it is shorter than N-1, then the
unused bytes are padded by further 00h's). The character set is normal ASCII
for non-Japan (see Region/Version entry in header), and 2-byte SHIFT-JIS for
Japanese long-titles (=max 16 2-byte chars) with values as so:

  00h          --> end-byte
  81h,40h      --> SPC
  81h,43h..97h --> punctuation marks
  82h,4Fh..58h --> "0..9"
  82h,60h..79h --> "A..Z"
  82h,81h..9Ah --> "a..z"

And 1-byte chars for Japanese short-titles,

  00     = end-byte
  01     = spc
  02..0B = 0..9
  0C..AF = japanese
  B0..B4 = dash, male, female, comma, round-dot
  B5..C0 = !"%&~?/+-:.'
  C1..DA = A..Z
  DB..DF = unused (blank)
  E0..E5 = japanese
  E6..FF = a..z
  N/A    = #$()*;<=>@[\]^_`{|}

Additionally to the Main-Title, optional Sub-Titles for each strip can be
included (see Sub-Title Flag in header). If enabled, then ALL strip titles are
included in each strip (allowing to show a preview of which strips have/haven't
been scanned yet).

The e-Reader can display maximum of 8 sub-titles, if the data consists of more
than 8 strips, then sub-titles aren't displayed (so it'd be waste of space to
include them in the dotcodes).

The Main Title gets clipped to 128 pixels width (that are, circa 22
characters), and, the e-Reader BIOS acts confused on multi-strip games with
Main Titles longer than 26 characters (so the full 33 bytes may be used only in
Japan; with 16bit charset).

If the title is empty (00h-filled), and there is only one card in the
application, then the application is started immediately. That, without
allowing the user to save it in FLASH memory.

Caution: Although shorter Titles do save memory, they do act unpleasant: the
text "(C) P-Letter" will be displayed at the bottom of the loading screen.

On Japanese/Original, 8bit sub-titles can be up to 18 characters (without any
end-byte) (or less when stats are enabled, due to limited screen width).

Card Types (Primary Type.Bit0 and Region/Type.Bit12-15)

  00h..01h  Blank Screen (?)
  02h..03h  Dotcode Application with 17byte-title, with stats, load music A
  04h..05h  Dotcode Application with 17byte-title, with stats, load music B
  06h..07h  P-Letter Attacks
  08h..09h  Construction Escape
  0Ah..0Bh  Construction Action
  0Ch..0Dh  Construction Melody Box
  0Eh       Dotcode Application with 33byte-title, without stats, load music A
  0Fh       Game specific cards
  10h..1Dh  P-Letter Viewer
  1Eh..1Fh  Same as 0Eh and 0Fh (see above)

The 'Application' types are meant to be executable GBA/Z80/NES programs.
```
