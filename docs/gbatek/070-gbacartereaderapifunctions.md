# GBA Cart e-Reader API Functions

Source: https://problemkaputt.de/gbatek.htm#gbacartereaderapifunctions

```
Z80 Interface (Special Opcodes)

  db 76h       ;Wait8bit A
  db D3h,xxh   ;Wait8bit xxh
  db C7h,xxh   ;RST0_xxh
  db CFh,xxh   ;RST8_xxh
  ld r,[00xxh]       ;get system values (addresses differ on jap/ori)
  ld r,[00C2h..C3h]  ;GetKeyStateSticky (jap/ori: 9F02h..9F03h)
  ld r,[00C4h..C5h]  ;GetKeyStateRaw    (jap/ori: 9F04h..9F05h)
  ld r,[00C0h..C1h]  ;see Exit and ExitRestart
  ld r,[00D0h..D3h]  ;see Mul16bit

For jap/ori, 9Fxxh isn't forwards compatible with jap/plus, so it'd be better
to check joypad via IoRead.

GBA Interface

  bx [30075FCh] ;ApiVector ;in: r0=func_no,r1,r2,r3,[sp+0],[sp+4],[sp+8]=params
  bx lr         ;Exit      ;in: r0 (0=Restart, 2=To_Menu)

Wait8bit/Wait16bit

The various Wait opcodes and functions are waiting as many frames as specified.
Many API functions have no effect until the next Wait occurs.

Z80 RST0_xxh Functions / GBA Functions 02xxh

  RST0_00h FadeIn, A speed, number of frames (0..x)
  RST0_01h FadeOut
  RST0_02h BlinkWhite
  RST0_03h  (?)
  RST0_04h  (?) blend_func_unk1
  RST0_05h  (?)
  RST0_06h  (?)
  RST0_07h  (?)
  RST0_08h  (?)
  RST0_09h  (?) _020264CC_check
  RST0_0Ah  (?) _020264CC_free
  RST0_0Bh N/A (bx 0)
  RST0_0Ch N/A (bx 0)
  RST0_0Dh N/A (bx 0)
  RST0_0Eh N/A (bx 0)
  RST0_0Fh N/A (bx 0)
  RST0_10h LoadSystemBackground, A number of background (1..101), E bg# (0..3)
  RST0_11h SetBackgroundOffset, A=bg# (0..3), DE=X, BC=Y
  RST0_12h SetBackgroundAutoScroll
  RST0_13h SetBackgroundMirrorToggle
  RST0_14h  (?)
  RST0_15h  (?)
  RST0_16h  (?) write_000000FF_to_02029494_
  RST0_17h  (?)
  RST0_18h  (?)
  RST0_19h SetBackgroundMode, A=mode (0..2)
  RST0_1Ah  (?)
  RST0_1Bh  (?)
  RST0_1Ch  (?)
  RST0_1Dh  (?)
  RST0_1Eh  (?)
  RST0_1Fh  (?)
  RST0_20h LayerShow
  RST0_21h LayerHide
  RST0_22h  (?)
  RST0_23h  (?)
  RST0_24h ... [20264DCh+A*20h+1Ah]=DE, [20264DCh+A*20h+1Ch]=BC
  RST0_25h  (?)
  RST0_26h  (?)
  RST0_27h  (?)
  RST0_28h  (?)
  RST0_29h  (?)
  RST0_2Ah  (?)
  RST0_2Bh  (?)
  RST0_2Ch  (?)
  RST0_2Dh LoadCustomBackground, A bg# (0..3), DE pointer to struct_background,
           max. tile data size = 3000h bytes, max. map data size = 1000h bytes
  RST0_2Eh GBA: N/A - Z80: (?)
  RST0_2Fh  (?)
  RST0_30h CreateSystemSprite, - -   (what "- -" ???)
  RST0_31h SpriteFree, HL sprite handle
  RST0_32h SetSpritePos, HL=sprite handle, DE=X, BC=Y
  RST0_33h  (?) sprite_unk2
  RST0_34h SpriteFrameNext
  RST0_35h SpriteFramePrev
  RST0_36h SetSpriteFrame, HL=sprite handle, E=frame number (0..x)
  RST0_37h  (?) sprite_unk3
  RST0_38h  (?) sprite_unk4
  RST0_39h SetSpriteAutoMove, HL=sprite handle, DE=X, BC=Y
  RST0_3Ah  (?) sprite_unk5
  RST0_3Bh  (?) sprite_unk6
  RST0_3Ch SpriteAutoAnimate
  RST0_3Dh  (?) sprite_unk7
  RST0_3Eh SpriteAutoRotateUntilAngle
  RST0_3Fh SpriteAutoRotateByAngle
  RST0_40h SpriteAutoRotateByTime
  RST0_41h  (?) sprite_unk8
  RST0_42h SetSpriteAutoMoveHorizontal
  RST0_43h SetSpriteAutoMoveVertical
  RST0_44h  (?) sprite_unk9
  RST0_45h SpriteDrawOnBackground
  RST0_46h SpriteShow, HL=sprite handle
  RST0_47h SpriteHide, HL=sprite handle
  RST0_48h SpriteMirrorToggle
  RST0_49h  (?) sprite_unk10
  RST0_4Ah  (?) sprite_unk11
  RST0_4Bh  (?) sprite_unk12
  RST0_4Ch GetSpritePos
  RST0_4Dh CreateCustomSprite
  RST0_4Eh  (?)
  RST0_4Fh  (?) sprite_unk14
  RST0_50h  (?) sprite_unk15
  RST0_51h  (?) sprite_unk16
  RST0_52h  (?) sprite_unk17
  RST0_53h  (?) sprite_unk18
  RST0_54h  (?)
  RST0_55h  (?) sprite_unk20
  RST0_56h  (?)
  RST0_57h SpriteMove
  RST0_58h  (?) sprite_unk22
  RST0_59h  (?) sprite_unk23
  RST0_5Ah  (?) sprite_unk24
  RST0_5Bh SpriteAutoScaleUntilSize, C=speed (higher value is slower),
           HL=sprite handle, DE=size (0100h = normal size,
           lower value = larger, higher value = smaller)
  RST0_5Ch SpriteAutoScaleBySize
  RST0_5Dh SpriteAutoScaleWidthUntilSize
  RST0_5Eh SpriteAutoScaleHeightBySize
  RST0_5Fh  (?)
  RST0_60h  (?)
  RST0_61h  (?)
  RST0_62h  (?)
  RST0_63h  (?)
  RST0_64h hl=[[2024D28h+a*4]+12h]
  RST0_65h  (?) sprite_unk25
  RST0_66h SetSpriteVisible, HL=sprite handle, E=(0=not visible, 1=visible)
  RST0_67h  (?) sprite_unk26
  RST0_68h  (?) set_sprite_unk27
  RST0_69h  (?) get_sprite_unk27
  RST0_6Ah  (?)
  RST0_6Bh  (?)
  RST0_6Ch  (?)
  RST0_6Dh  (?)
  RST0_6Eh hl=[hl+000Ah]  ;r0=[r1+0Ah]
  RST0_6Fh  (?)
  RST0_70h  (?)
  RST0_71h  (?)
  RST0_72h  (?)
  RST0_73h  (?)
  RST0_74h  (?)
  RST0_75h  (?)
  RST0_76h  (?)
  RST0_77h  (?)
  RST0_78h  (?)
  RST0_79h  (?)
  RST0_7Ah  (?)
  RST0_7Bh  (?)
  RST0_7Ch  (?) _0202FD2C_unk12
  RST0_7Dh Wait16bit ;HL=num_frames (16bit variant of Wait8bit opcode/function)
  RST0_7Eh SetBackgroundPalette, HL=src_addr, DE=offset, C=num_colors (1..x)
  RST0_7Fh GetBackgroundPalette(a,b,c)
  RST0_80h SetSpritePalette, HL=src_addr, DE=offset, C=num_colors (1..x)
  RST0_81h GetSpritePalette(a,b,c)
  RST0_82h ClearPalette
  RST0_83h  (?) _0202FD2C_unk11
  RST0_84h  (?)
  RST0_85h  (?)
  RST0_86h  (?)
  RST0_87h  (?) _0202FD2C_unk8
  RST0_88h  (?) _0202FD2C_unk7
  RST0_89h  (?)
  RST0_8Ah  (?) _0202FD2C_unk6
  RST0_8Bh  (?) _0202FD2C_unk5
  RST0_8Ch GBA: N/A - Z80: (?)
  RST0_8Dh GBA: N/A - Z80: (?)
  RST0_8Eh  (?)
  RST0_8Fh WindowHide
  RST0_90h CreateRegion, H=bg# (0..3), L=palbank# (0..15),
           D,E,B,C=x1,y1,cx,cy (in tiles), return: n/a (no$note: n/a ???)
  RST0_91h SetRegionColor
  RST0_92h ClearRegion
  RST0_93h SetPixel
  RST0_94h GetPixel
  RST0_95h DrawLine
  RST0_96h DrawRect
  RST0_97h  (?) _0202FD2C_unk4
  RST0_98h SetTextColor, A=region handle, D=color foreground (0..15),
           E=color background (0..15)
  RST0_99h DrawText, A=region handle, BC=pointer to text, D=X, E=Y
           (non-japan uses ASCII text, but japanese e-reader's use STH ELSE?)
  RST0_9Ah SetTextSize
  RST0_9Bh  (?) RegionUnk7
  RST0_9Ch  (?) _0202FD2C_unk3
  RST0_9Dh  (?) _0202FD2C_unk2
  RST0_9Eh  (?) _0202FD2C_unk1
  RST0_9Fh Z80: (?) - GBA: SetBackgroundModeRaw
  RST0_A0h  (?)
  RST0_A1h  (?)
  RST0_A2h  (?) RegionUnk6
  RST0_A3h GBA: N/A - Z80: (?)
  RST0_A4h GBA: N/A - Z80: (?)
  RST0_A5h  (?)
  RST0_A6h  (?)
  RST0_A7h  (?)
  RST0_A8h  (?)
  RST0_A9h  (?)
  RST0_AAh  (?)
  RST0_ABh  (?)
  RST0_ACh  (?)
  RST0_ADh  (?) RegionUnk5
  RST0_AEh [202FD2Ch+122h]=A
  RST0_AFh [202FD2Ch+123h]=A
  RST0_B0h [202FD2Ch+124h]=A
  RST0_B1h  (?)
  RST0_B2h  (?)
  RST0_B3h GBA: N/A - Z80: Sqrt   ;hl=sqrt(hl)
  RST0_B4h GBA: N/A - Z80: ArcTan ;hl=ArcTan2(hl,de)
  RST0_B5h Sine                   ;hl=sin(a)*de
  RST0_B6h Cosine                 ;hl=cos(a)*de
  RST0_B7h  (?)
  RST0_B8h  (?)
  RST0_B9h N/A (bx 0)
  RST0_BAh N/A (bx 0)
  RST0_BBh N/A (bx 0)
  RST0_BCh N/A (bx 0)
  RST0_BDh N/A (bx 0)
  RST0_BEh N/A (bx 0)
  RST0_BFh N/A (bx 0)
  Below Non-Japan and Japan/Plus only (not Japan/Ori)
  RST0_C0h GetTextWidth(a,b)
  RST0_C1h GetTextWidthEx(a,b,c)
  RST0_C2h  (?)
  RST0_C3h Z80: N/A (bx 0) - GBA: (?)
  RST0_C4h  (?)
  RST0_C5h  (?)
  RST0_C6h  (?)
  RST0_C7h  (?)
  RST0_C8h  (?)
  RST0_C9h  (?)
  RST0_CAh  (?)
  RST0_CBh  (?)
  RST0_CCh  (?)
  RST0_CDh N/A (bx lr)
  RST0_CEh ;same as RST0_3Bh, but with 16bit mask
  RST0_CFh ;same as RST0_3Eh, but with 16bit de
  RST0_D0h ;same as RST0_3Fh, but with 16bit de
  RST0_D1h ;same as RST0_5Bh, but with 16bit de
  RST0_D2h ;same as RST0_5Ch, but with 16bit de
  RST0_D3h ;same as RST0_5Dh, but with 16bit de
  RST0_D4h ;same as RST0_5Eh, but with 16bit de
  RST0_D5h  (?)
  RST0_D6h  (?)
  RST0_D7h ;[202FD2Ch+125h]=A
  RST0_D8h  (?)
  RST0_D9h  (?)
  RST0_DAh  (?)
  RST0_DBh ;A=[3003E51h]
  RST0_DCh ;[3004658h]=01h
  RST0_DDh DecompressVPKorNonVPK
  RST0_DEh FlashWriteSectorSingle(a,b)
  RST0_DFh FlashReadSectorSingle(a,b)
  RST0_E0h SoftReset
  RST0_E1h GetCartridgeHeader     ;[hl+0..BFh]=[8000000h..80000BFh]
  RST0_E2h GBA: N/A - Z80: bx hl  ;in: hl=addr, af,bc,de,sp=param, out: a
  RST0_E3h Z80: N/A (bx 0) - GBA: (?)
  RST0_E4h  (?)
  RST0_E5h  (?)
  RST0_E6h  (?)
  RST0_E7h  (?)
  RST0_E8h  (?)
  RST0_E9h ;[2029498h]=0000h
  RST0_EAh Z80: N/A (bx 0) - GBA: InitMemory(a)
  RST0_EBh  (?) BL_irq_sio_dma3
  RST0_ECh ;hl = [3003E30h]*100h + [3003E34h]
  RST0_EDh FlashWriteSectorMulti(a,b,c)
  RST0_EEh FlashReadPart(a,b,c)
  RST0_EFh ;A=((-([2029416h] xor 1)) OR (+([2029416h] xor 1))) SHR 31
  RST0_F0h  (?) _unk1
  RST0_F1h RandomInit     ;in: hl=random_seed
  RST0_F2h                         (?)
  Below Japan/Plus only
  RST0_F3h  (?)
  RST0_F4h  (?)
  RST0_F5h  (?)
  RST0_F6h  (?)
  RST0_F7h GBA: N/A - Z80: (?)
  Below is undefined/garbage (values as so in Z80 mode)
  Jap/Ori: RST0_C0h      N/A (bx 0)
  Jap/Ori: RST0_C1h..FFh Overlaps RST8 jump list
  Non-Jap: RST0_F3h..FFh Overlaps RST8 jump list
  Jap/Pls: RST0_F8h..FFh Overlaps RST8 jump list

Z80 RST8_xxh Functions / GBA Functions 01xxh

  RST8_00h GBA: N/A - Z80: Exit       ;[00C0h]=a ;(1=restart, 2=exit)
  RST8_01h GBA: N/A - Z80: Mul8bit    ;hl=a*e
  RST8_02h GBA: N/A - Z80: Mul16bit   ;hl=hl*de, s32[00D0h]=hl*de
  RST8_03h Div                        ;hl=hl/de
  RST8_04h DivRem                     ;hl=hl mod de
  RST8_05h PlaySystemSound            ;in: hl=sound_number
  RST8_06h  (?) sound_unk1
  RST8_07h Random8bit                 ;a=random(0..FFh)
  RST8_08h SetSoundVolume
  RST8_09h BcdTime                    ;[de+0..5]=hhmmss(hl*bc)
  RST8_0Ah BcdNumber                  ;[de+0..4]=BCD(hl), [de+5]=00h
  RST8_0Bh IoWrite                    ;[4000000h+hl]=de
  RST8_0Ch IoRead                     ;de=[4000000h+hl]
  RST8_0Dh GBA: N/A - Z80:  (?)
  RST8_0Eh GBA: N/A - Z80:  (?)
  RST8_0Fh GBA: N/A - Z80:  (?)
  RST8_10h GBA: N/A - Z80:  (?)
  RST8_11h DivSigned                  ;hl=hl/de, signed
  RST8_12h RandomMax                  ;a=random(0..a-1)
  RST8_13h SetSoundSpeed
  RST8_14h  hl=[202FD20h]=[2024CACh]
  RST8_15h  hl=[2024CACh]-[202FD20h]
  RST8_16h SoundPause
  RST8_17h SoundResume
  RST8_18h PlaySystemSoundEx
  RST8_19h IsSoundPlaying
  RST8_1Ah  (?)
  RST8_1Bh  (?)
  RST8_1Ch  (?)
  RST8_1Dh GetExitCount               ;a=[2032D34h]
  RST8_1Eh Permille                   ;hl=de*1000/hl
  RST8_1Fh GBA: N/A - Z80: ExitRestart;[2032D38h]=a, [00C0h]=0001h  ;a=?
  RST8_20h GBA: N/A - Z80: WaitJoypad ;wait until joypad<>0, set hl=joypad
  RST8_21h GBA: N/A - Z80:  (?)
  RST8_22h  (?) _sound_unk7
  RST8_23h  (?) _sound_unk8
  RST8_24h  (?) _sound_unk9
  RST8_25h  (?) _sound_unk10
  RST8_26h Mosaic     ;bg<n>cnt.bit6=a.bit<n>, [400004Ch]=de
  RST8_27h  (?)
  RST8_28h  (?)
  RST8_29h  (?)
  RST8_2Ah  (?) get_8bit_from_2030110h
  RST8_2Bh  (?)
  RST8_2Ch  (?) get_16bit_from_2030112h ;jap/ori: hl=[20077B2h]
  RST8_2Dh  (?) get_16bit_from_2030114h ;jap/ori: hl=[20077B4h]
  RST8_2Eh  (?)
  RST8_2Fh PlayCustomSound(a,b)
  Below not for Japanese/Original
  (the renumbered functions can be theoretically used on japanese/original)
  (but, doing so would blow forwards compatibility with japanese/plus)
  RST8_30h (ori: none)      GBA: N/A - Z80: (?)
  RST8_31h (ori: none)      PlayCustomSoundEx(a,b,c)
  RST8_32h (ori: RST8_30h)  BrightnessHalf   ;[4000050h]=00FFh,[4000054h]=0008h
  RST8_33h (ori: RST8_31h)  BrightnessNormal ;[4000050h]=0000h
  RST8_34h (ori: RST8_32h)  N/A (bx lr)
  RST8_35h (ori: RST8_33h)   (?)
  RST8_36h (ori: RST8_34h)  ResetTimer ;[400010Ch]=00000000h, [400010Eh]=A+80h
  RST8_37h (ori: RST8_35h)  GetTimer   ;hl=[400010Ch]
  RST8_38h (ori: none)      GBA: N/A - Z80:  (?)
  Below is undefined/reserved/garbage (values as so in Z80 mode)
  (can be used to tweak jap/ori to start GBA-code from inside of Z80-code)
  (that, after relocating code to 3000xxxh via DMA via IoWrite function)
  RST8_39h (ori: RST8_36h)  bx 0140014h
  RST8_3Ah (ori: RST8_37h)  bx 3E700F0h
  RST8_3Bh (ori: RST8_38h)  bx 3E70000h+1
  RST8_3Ch (ori: RST8_39h)  bx 3E703E6h+1
  RST8_3Dh (ori: RST8_3Ah)  bx 3E703E6h+1
  RST8_3Eh (ori: RST8_3Bh)  bx 3E703E6h+1
  RST8_3Fh (ori: RST8_3Ch)  bx 3E703E6h+1
  40h-FFh  (ori: 3Dh-FFh)   bx ...

GBA Functions 03xxh (none such in Z80 mode)

  RSTX_00h Wait8bit  ;for 16bit: RST0_7Dh
  RSTX_01h GetKeyStateSticky()
  RSTX_02h GetKeyStateRaw()
  RSTX_03h  (?)
  RSTX_04h  (?)
```
