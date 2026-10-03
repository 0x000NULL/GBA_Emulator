# Library compatibility (1543 titles, 2026-10-01)

Boot sweep by `scripts/triage.py` (harness `gba-test --triage`): every ROM runs 3000 frames (50 s) under the
HLE BIOS with Start pressed in pairs at frames 700+800/1300+1400/1900+2000/2500+2600 and A at 1150/2350, then the final picture, its
motion, the audio and the CPU state are classified. `ok` = a moving, coloured picture with audio. This is a boot
and title/menu check, not a play-through; the scripted gameplay goldens live in `tests/compat.toml`.
The sweep ran before the cartridge GPIO port existed (2026-10-03): the ten real-time-clock carts and the
three Boktai solar-sensor carts booted then too, and their notes say what they gain now.

| Category | Titles | Meaning |
|---|---|---|
| ok | 1528 | moving picture with colour and audio |
| waiting | 10 | reached a menu or entry screen that waits for a key the sweep does not press (verified by eye) |
| silent | 0 | moving picture, no audio in the first 40 s (see the per-title note) |
| needs-bios-sound-driver | 0 | runs, but its music needs a BIOS sound function the HLE BIOS does not provide (SWI 20h-24h) |
| static | 0 | a picture that stopped changing before the last 20 s |
| frozen | 0 | picture stopped and the CPU is idle or spinning |
| black | 3 | no picture at all |
| needs-hardware | 1 | needs cartridge hardware the emulator does not provide |
| not-a-game | 1 | not a GBA program (expansion data) |
| crash | 0 | the harness failed (panic / unreadable ROM) |

| Title | Code | Save | Result | Note |
|---|---|---|---|---|
| 007 - Everything or Nothing (USA, Europe) (En,Fr,De) | BJBE | Eeprom | ok |  |
| 007 - NightFire (USA, Europe) (En,Fr,De) | A7OE | Eeprom | ok |  |
| 2 Game Pack! - Matchbox Missions - Emergency Response + Air, Land and Sea Rescue (USA) | BB4E | None | ok |  |
| 2 Game Pack! - Uno + Skip-Bo (USA) | BUQE | None | ok |  |
| 2 Games in One! - Dr. Mario + Puzzle League (USA) | BZPE | Eeprom | ok |  |
| 2 Games in One! - Gauntlet + Rampart (USA) | B69E | None | ok |  |
| 2 Games in One! - Marble Madness + Klax (USA) | B68E | None | ok |  |
| 2 Games in One! - Paperboy + Rampage (USA) | B6BE | None | ok |  |
| 2 Games in One! - Spy Hunter + Super Sprint (USA) | B6AE | None | ok |  |
| 2006 FIFA World Cup - Germany 2006 (USA, Europe) (En,Fr,De,Es,It) | B6WE | Eeprom | ok |  |
| 2K Sports - Major League Baseball 2K7 (USA) | B2YE | None | ok |  |
| 3 Game Pack! - Candy Land + Chutes and Ladders + Original Memory Game (USA) | BC4E | None | ok |  |
| 3 Game Pack! - Ker Plunk! + Toss Across + Tip It (USA) | BXCE | None | ok |  |
| 3 Game Pack! - Mouse Trap + Simon + Operation (USA) | B3OE | None | ok |  |
| 3 Game Pack! - The Game of Life + Payday + Yahtzee (USA) | B3UE | None | ok |  |
| 3 Games in One - Darts + Roll-a-Ball + Shuffle Bowl (USA) | BRQE | None | ok |  |
| 3 Games in One! - Breakout + Centipede + Warlords (USA) | B6ZE | None | ok |  |
| 3 Games in One! - Super Breakout + Millipede + Lunar Lander (USA) | B62E | None | ok |  |
| 3 Games in One! - Yars' Revenge + Asteroids + Pong (USA) | B64E | None | ok |  |
| Ace Combat Advance (USA, Europe) | BAEE | None | ok |  |
| Ace Lightning (Europe) | ALXP | Eeprom | ok |  |
| Action Man - Robot Atak (Europe) (En,Fr,De,Es,It) | BACP | None | ok |  |
| Activision Anthology (USA) | BAVE | Eeprom | ok |  |
| Advance Guardian Heroes (USA) | BGCE | Eeprom | ok |  |
| Advance Wars (USA) (Rev 1) | AWRE | Flash64K | ok |  |
| Advance Wars 2 - Black Hole Rising (USA) | AW2E | Flash64K | ok |  |
| Adventure of Tokyo Disney Sea (Japan) | ADEJ | Sram32K | ok |  |
| Adventures of Jimmy Neutron Boy Genius vs. Jimmy Negatron, The (USA, Europe) | AJXE | None | ok |  |
| Adventures of Jimmy Neutron Boy Genius, The - Attack of the Twonkies (USA, Europe) | BJYE | Eeprom | ok |  |
| Adventures of Jimmy Neutron Boy Genius, The - Jet Fusion (USA, Europe) | BJNE | None | ok |  |
| Aero the Acro-Bat (USA) | AAOE | Eeprom | ok |  |
| Agassi Tennis Generation (USA) | ACEE | None | ok |  |
| Agent Hugo - Roborumble (Europe) (En,Fr,De,Es,It,Nl,Pt,Sv,No,Da,Fi) | BHQP | None | ok |  |
| Aggressive Inline (USA) | AILE | None | ok |  |
| AirForce Delta Storm (USA) (En,Ja,Fr,De) | AAKE | Eeprom | ok |  |
| Aka-chan Doubutsuen (Japan) (Rev 1) | BAZJ | Eeprom | ok |  |
| Aladdin (USA) (En,Fr,De,Es) | BADE | Eeprom | ok |  |
| Aleck Bordon Adventure - Tower & Shaft Advance (Japan) | BABJ | Eeprom | ok |  |
| Alex Rider - Stormbreaker (USA) | BAWE | Eeprom | ok |  |
| Alien Hominid (Europe) (En,Fr,De,Es,It) | BAHP | Eeprom | ok |  |
| Alienators - Evolution Continues (USA, Europe) | AEVE | None | ok |  |
| All Grown Up! - Express Yourself (USA, Europe) | BALE | Eeprom | ok |  |
| All-Star Baseball 2003 (USA) | AA3E | Sram32K | ok |  |
| All-Star Baseball 2004 Featuring Derek Jeter (USA) | AA7E | Sram32K | ok |  |
| Altered Beast - Guardian of the Realms (USA) | AARE | Eeprom | ok |  |
| Amazing Virtual Sea-Monkeys, The (USA) | A7ME | None | ok |  |
| American Bass Challenge (USA) | AABE | Eeprom | ok |  |
| American Dragon - Jake Long - Rise of the Huntsclan (USA, Europe) (En,Fr,De,Es,It) | BAPE | Eeprom | ok |  |
| American Idol (USA) | BIDE | None | ok |  |
| American Tail, An - Fievel's Gold Rush (USA) (En,Es) | AFGE | None | ok |  |
| Angel Collection - Mezase! Gakuen no Fashion Leader (Japan) | AFNJ | Eeprom | ok |  |
| Angel Collection 2 - Pichimo ni Narou (Japan) | BECJ | Eeprom | ok |  |
| Angelique (Japan) | AAGJ | Eeprom | ok |  |
| Animal Mania - Dokidoki Aishou Check (Japan) | AANJ | Sram32K | ok |  |
| Animal Snap - Rescue Them 2 by 2 (USA) | AAQE | None | ok |  |
| Animal Yokochou - Doki Doki Kyuushutsu Daisakusen! no Maki (Japan) | BAYJ | Sram32K | ok |  |
| Animal Yokochou - Doki Doki Shinkyuu Shiken! no Maki (Japan) | BAXJ | Sram32K | ok |  |
| Animaniacs - Lights, Camera, Action! (Europe) (En,Fr,De,Es,It) | ANIP | None | ok |  |
| Ant Bully, The (USA) (En,Fr) | BUYE | Eeprom | ok |  |
| Antz - Extreme Racing (USA) | ANUE | None | ok |  |
| Ao-Zora to Nakama-tachi - Yume no Bouken (Japan) | AAZJ | Eeprom | ok |  |
| Archer Maclean's 3D Pool (USA) | BPLE | None | ok |  |
| Arctic Tale (USA) | BB5E | None | ok |  |
| Army Men - Operation Green (USA) (En,Fr,De,Es,It) | AMEE | None | ok |  |
| Army Men - Turf Wars (USA) | AY3E | Eeprom | ok |  |
| Army Men Advance (USA, Europe) (En,Fr,De,Es,It) | ASAE | None | ok |  |
| Around the World in 80 Days (USA) | B8DE | None | ok |  |
| Arthur and the Invisibles (USA) (En,Fr,Es) | B2NE | Eeprom | ok |  |
| Ashita no Joe - Makka ni Moeagare! (Japan) | BAMJ | Sram32K | ok |  |
| Asterix & Obelix - Bash Them All! (Europe) (En,Fr,De,Es,It,Nl) | AOBP | None | ok |  |
| Asterix & Obelix XXL (Europe) (En,Fr,De,Es,It,Nl) | BLXP | Eeprom | ok |  |
| Astro Boy - Omega Factor (USA) (En,Ja,Fr,De,Es,It) | BTAE | Eeprom | ok |  |
| Atari Anniversary Advance (USA) | AAVE | None | ok | music through the HLE BIOS sound driver (2026-10-01); pinned in tests/compat.toml |
| Atlantis - The Lost Empire (USA, Europe) | ATLE | None | ok |  |
| Atomic Betty (USA, Europe) | BETE | None | ok |  |
| ATV - Quad Power Racing (USA, Europe) | AQRE | None | ok |  |
| ATV - Thunder Ridge Riders (USA) | B3BE | None | ok |  |
| Avatar - The Last Airbender (USA) | BQZE | Eeprom | ok |  |
| Avatar - The Last Airbender - The Burning Earth (USA) | BBWE | None | ok |  |
| Azumanga Daiou Advance (Japan) | AZAJ | Eeprom | ok |  |
| Babar to the Rescue (USA) (En,Fr,Es) | BBVE | Eeprom | ok |  |
| Back to Stone (USA) (En,Fr) | BBCE | None | ok |  |
| Back Track (USA, Europe) | ABKE | Eeprom | ok |  |
| Backyard Baseball (USA) | ACKE | Eeprom | ok |  |
| Backyard Baseball 2006 (USA) | BCYE | Eeprom | ok |  |
| Backyard Basketball (USA) | AYBE | Eeprom | ok |  |
| Backyard Football (USA) | AYFE | Eeprom | ok |  |
| Backyard Football 2006 (USA) | BYFE | Eeprom | ok |  |
| Backyard Hockey (USA) | BYHE | Eeprom | ok |  |
| Backyard Skateboarding (USA) | BS6E | Eeprom | ok |  |
| Backyard Sports - Baseball 2007 (USA) | BC7E | Eeprom | ok |  |
| Backyard Sports - Basketball 2007 (USA) | BB7E | Eeprom | ok |  |
| Backyard Sports - Football 2007 (USA) | BF7E | Eeprom | ok |  |
| Bakuten Shoot Beyblade - Gekitou! Saikyou Blader (Japan) | AHEJ | Sram32K | ok |  |
| Bakuten Shoot Beyblade 2002 - Gekisen! Team Battle!! Kouryuu no Shou - Daichi Hen (Japan) | A3EJ | Sram32K | ok |  |
| Bakuten Shoot Beyblade 2002 - Gekisen! Team Battle!! Seiryuu no Shou - Takao Hen (Japan) | A3WJ | Sram32K | ok |  |
| Bakuten Shoot Beyblade 2002 - Ikuze! Bakutou! Chou Jiryoku Battle!! (Japan) | AB8J | Sram32K | ok |  |
| Baldur's Gate - Dark Alliance (USA) | BGDE | Eeprom | ok |  |
| Ballistic - Ecks vs. Sever (USA) | AEEE | None | ok |  |
| Banjo-Kazooie - Grunty's Revenge (USA, Europe) | BKZE | Eeprom | ok |  |
| Banjo-Pilot (USA) | BAJE | Eeprom | ok |  |
| Barbie - The Princess and the Pauper (USA) | BAUE | None | ok |  |
| Barbie and the Magic of Pegasus (USA) | BE5E | None | ok |  |
| Barbie as The Island Princess (USA) | BBNE | Eeprom | ok |  |
| Barbie Diaries, The - High School Mystery (USA) | BBIE | Eeprom | ok |  |
| Barbie Horse Adventures - Blue Ribbon Race (USA) | AI8E | None | ok |  |
| Barbie in the 12 Dancing Princesses (USA) | BB3E | None | ok |  |
| Barbie Software - Groovy Games (USA) | AVBE | None | ok |  |
| Barnyard (USA) | BBYE | Eeprom | ok |  |
| Baseball Advance (USA) | ABPE | Sram32K | ok |  |
| Bass Tsuri Shiyouze! - Tournament wa Senryaku da! (Japan) | AZBJ | Eeprom | ok |  |
| Batman - Rise of Sin Tzu (USA) (En,Fr,Es) | BATE | None | ok |  |
| Batman - Vengeance (USA) (En,Fr,Es) | ABTE | None | ok |  |
| Batman Begins (USA, Europe) (En,Fr,De,Es,It,Nl) | BBGE | Eeprom | ok |  |
| Battle B-Daman (USA) | BDXE | Eeprom | ok |  |
| Battle B-Daman - Fire Spirits! (USA) | BBME | Eeprom | ok |  |
| Battle X Battle - Kyodai Gyo Densetsu (Japan) | BBFJ | Eeprom | ok |  |
| BattleBots - Beyond the BattleBox (USA) | ABEE | Eeprom | ok |  |
| BattleBots - Design & Destroy (USA) | BBDE | None | ok |  |
| BB Ball (Japan) | A8LJ | Sram32K | ok |  |
| Beast Shooter - Mezase Beast King! (Japan) | AH5J | Sram32K | ok |  |
| Bee Game, The (USA) | BKFE | Eeprom | ok |  |
| Berenstain Bears and the Spooky Old Tree, The (USA) | BBOE | None | ok |  |
| Best Play Pro Yakyuu (Japan) | A8YJ | Flash64K | ok |  |
| Beyblade G-Revolution (USA) | BB2E | Eeprom | ok |  |
| Beyblade V-Force - Ultimate Blader Jam (USA) | BEYE | Eeprom | ok |  |
| Bibi Blocksberg - Der Magische Hexenkreis (Germany) | BBXD | None | ok |  |
| Bibi und Tina - Ferien auf dem Martinshof (Germany) (Rev 1) | BUXD | Eeprom | ok |  |
| Bible Game, The (USA) | BIBE | None | ok |  |
| Big Mutha Truckers (USA) | B63E | Eeprom | ok |  |
| Bionicle (USA) | BIOE | None | ok |  |
| Bionicle - Matoran Adventures (USA, Europe) (En,Fr,De,Es,It,Nl,Sv,Da) | A5AE | Eeprom | ok |  |
| Bionicle - Maze of Shadows (USA) | BILE | Eeprom | ok |  |
| Bionicle Heroes (USA) (En,Fr,De,Es,It,Da) | BIHE | Eeprom | ok |  |
| bit Generations - Boundish (Japan) (En) | BVDJ | Sram32K | ok |  |
| bit Generations - Coloris (Japan) (En) | BVAJ | Sram32K | ok |  |
| bit Generations - Dialhex (Japan) (En) | BVBJ | Sram32K | ok |  |
| bit Generations - Digidrive (Japan) (En) | BVHJ | Sram32K | ok |  |
| bit Generations - Dotstream (Japan) (En) | BVCJ | Sram32K | ok |  |
| bit Generations - Orbital (Japan) (En) | BVEJ | Sram32K | ok |  |
| bit Generations - Soundvoyager (Japan) (En) | BVGJ | Sram32K | ok |  |
| Black Belt Challenge (Europe) | AB6P | Eeprom | ok |  |
| Black Black - Bura Bura (Japan) | AWEJ | Sram32K | ok |  |
| Black Matrix Zero (Japan) | AXBJ | Eeprom | ok |  |
| Blackthorne (USA) | AQXE | Eeprom | ok |  |
| Blades of Thunder (USA) | BBHE | None | ok |  |
| Bleach Advance - Kurenai ni Somaru Soul Society (Japan) | BLEJ | Flash64K | ok |  |
| Blender Bros. (USA) | ABRE | Sram32K | ok |  |
| BMX Trick Racer (USA) | AT9E | Eeprom | ok |  |
| Board Game Classics (USA) | B6EE | None | ok |  |
| Boboboubo Boubobo - 9 Kyoku Senshi Gag Yuugou (Japan) | BB9J | Sram32K | ok |  |
| Boboboubo Boubobo - Bakutou Hajike Taisen (Japan) | BOSJ | Sram32K | ok |  |
| Boboboubo Boubobo - Maji de!! Shinken Battle (Japan) | BOBJ | Sram32K | ok |  |
| Boboboubo Boubobo - Ougi 87.5 Bakuretsu Hanage Shinken (Japan) | A8VJ | Sram32K | ok |  |
| Boktai - The Sun Is in Your Hand (USA) | U3IE | Eeprom | ok | cartridge RTC + solar sensor (since 2026-10-03; pinned in compat.toml) |
| Boktai 2 - Solar Boy Django (USA) | U32E | Eeprom | ok | cartridge RTC + solar sensor (since 2026-10-03) |
| Boku wa Koukuu Kanseikan (Japan) (Rev 1) | ABCJ | Sram32K | ok |  |
| Bomberman Jetters - Densetsu no Bomberman (Japan) | AJZJ | Sram32K | ok |  |
| Bomberman Jetters - Game Collection (Japan) | BOMJ | Eeprom | ok |  |
| Bomberman Max 2 - Blue Advance (USA) | AMHE | Eeprom | ok |  |
| Bomberman Max 2 - Red Advance (USA) | AMYE | Eeprom | ok |  |
| Bomberman Tournament (USA, Europe) | ABSE | Sram32K | ok |  |
| Bookworm (USA) | BKWE | Eeprom | ok |  |
| Bouken Yuuki Pluster World - Densetsu no Plust Gate (Japan) | APJJ | Sram32K | ok |  |
| Bouken Yuuki Pluster World - Densetsu no Plust Gate EX (Japan) | BPDJ | Sram32K | ok |  |
| Bouken Yuuki Pluster World - Pluston GP (Japan) | A2PJ | Sram32K | ok |  |
| Bouken-ou Beet - Busters Road (Japan) | BOVJ | Eeprom | ok |  |
| Boukyaku no Senritsu (Japan) | BBSJ | Eeprom | ok |  |
| Boulder Dash EX (USA) | ABDE | Eeprom | ok |  |
| Boxing Fever (USA, Europe) | ABOE | None | ok |  |
| Bratz (USA) (En,Fr,Es) | A2RE | Eeprom | ok |  |
| Bratz - Babyz (USA) | BBZE | None | ok |  |
| Bratz - Forever Diamondz (USA) | BXFE | Eeprom | waiting | menu waiting for input (Accept changes) |
| Bratz - Rock Angelz (USA, Europe) | BRRE | Eeprom | ok |  |
| Bratz - The Movie (USA) | BBUE | None | ok |  |
| Breath of Fire (USA) | ABFE | Sram32K | ok |  |
| Breath of Fire II (USA) | AB2E | Sram32K | ok |  |
| Britney's Dance Beat (USA) | ABYE | None | ok |  |
| Broken Sword - The Shadow of the Templars (USA) (En,Fr,De,Es,It) | ABJE | Eeprom | ok |  |
| Brother Bear (USA) | BBRE | Eeprom | ok |  |
| Bruce Lee - Return of the Legend (USA) | ALEE | Eeprom | ok |  |
| Bubble Bobble - Old & New (USA) | AONE | Eeprom | ok |  |
| Buffy the Vampire Slayer - Wrath of the Darkhul King (USA, Europe) | AVYE | Eeprom | ok |  |
| Butt-Ugly Martians - B.K.M. Battles (USA) | ABWE | None | ok |  |
| Cabbage Patch Kids - The Patch Puppy Rescue (USA) | BCGE | Eeprom | ok |  |
| Cabela's Big Game Hunter (USA) | A8HE | None | ok |  |
| Cabela's Big Game Hunter - 2005 Adventures (USA, Europe) | BG5E | Eeprom | ok |  |
| Caesars Palace Advance - Millennium Gold Edition (USA, Europe) | ACPE | None | ok |  |
| Calciobit (Japan) | BIXJ | Flash128K | ok |  |
| Camp Lazlo - Leaky Lake Games (USA) | BLCE | Eeprom | ok |  |
| Capcom Classics Mini Mix (USA) | BC6E | None | ok |  |
| Captain Tsubasa - Eikou no Kiseki (Japan) | AKYJ | Sram32K | ok |  |
| Car Battler Joe (USA) | ACBE | Eeprom | ok |  |
| Card Party (Japan) | A8CJ | None | ok |  |
| Cardcaptor Sakura - Sakura Card de Mini Game (Japan) | BK3J | Eeprom | ok |  |
| Cardcaptor Sakura - Sakura Card Hen - Sakura to Card to Otomodachi (Japan) (Rev 1) | BKSJ | Eeprom | ok |  |
| Care Bears - The Care Quest (USA) (En,Fr,Es) | BEAE | Eeprom | ok |  |
| Carrera Power Slide (Europe) (En,Fr,De,Es,It,Nl) | AEDP | Eeprom | ok |  |
| Cars (USA, Europe) | BCAE | Eeprom | ok |  |
| Cars - Mater-National Championship (USA) (En,Fr) | BCPE | None | ok |  |
| Cartoon Network Block Party (USA) | AC9E | None | ok |  |
| Cartoon Network Speedway (USA) | ANRE | None | ok |  |
| Casper (USA) (En,Fr,Es) | ACSE | None | ok |  |
| Castlevania - Aria of Sorrow (USA) | A2CE | Sram32K | ok |  |
| Castlevania - Circle of the Moon (USA) | AAME | Sram32K | ok |  |
| Castlevania - Harmony of Dissonance (USA) | ACHE | Sram32K | ok |  |
| Cat in the Hat, The (USA) | BCTE | None | ok |  |
| Catwoman (USA, Europe) (En,Fr,De,Es,It,Nl) | BCWE | Eeprom | ok |  |
| Catz (USA, Europe) | AN3E | Eeprom | ok |  |
| Charlie and the Chocolate Factory (USA) (En,Fr,Es,Nl) | BCFE | Eeprom | ok |  |
| Charlotte's Web (USA) (En,Fr,De,Es,It) | BCJE | Eeprom | ok |  |
| Cheetah Girls, The (USA) | BCQE | Eeprom | ok |  |
| Chessmaster (USA) | ACYE | Eeprom | ok |  |
| Chicken Little (USA, Europe) (En,Fr,De,Es,It,Nl) | BCHE | Eeprom | ok |  |
| Chicken Shoot (USA) | B6FE | None | ok |  |
| Chicken Shoot 2 (USA) | B6GE | None | ok |  |
| Chobits for Game Boy Advance - Atashi Dake no Hito (Japan) | AOCJ | Eeprom | ok |  |
| Chocobo Land - A Game of Dice (Japan) | A5BJ | Sram32K | ok |  |
| Chronicles of Narnia, The - The Lion, the Witch and the Wardrobe (USA, Europe) (En,Fr,De,Es,It,Nl,Sv,Da) | B2WE | Eeprom | ok |  |
| ChuChu Rocket! (USA) (En,Ja,Fr,De,Es) | ACRE | Flash64K | ok |  |
| CIMA - The Enemy (USA) | BCME | Eeprom | ok |  |
| Cinderella - Magical Dreams (USA) (En,Fr,De,Es,It) | BCDE | Eeprom | ok |  |
| Cinnamon Game Series 2 - Yume no Daibouken (Japan) | B2SJ | Eeprom | ok |  |
| Cinnamon Game Series 3 - Fuwafuwa Daisakusen (Japan) | B43J | Eeprom | ok |  |
| Cinnamoroll - Koko ni Iru yo (Japan) | BPSJ | Eeprom | ok |  |
| Classic NES Series - Bomberman (USA, Europe) | FBME | Eeprom | ok |  |
| Classic NES Series - Castlevania (USA) | FADE | Eeprom | ok |  |
| Classic NES Series - Donkey Kong (USA, Europe) | FDKE | Eeprom | ok |  |
| Classic NES Series - Dr. Mario (USA, Europe) | FDME | Eeprom | ok |  |
| Classic NES Series - Excitebike (USA, Europe) | FEBE | Eeprom | ok |  |
| Classic NES Series - Ice Climber (USA, Europe) | FICE | Eeprom | ok |  |
| Classic NES Series - Metroid (USA, Europe) | FMRE | Eeprom | ok |  |
| Classic NES Series - Pac-Man (USA, Europe) | FP7E | Eeprom | ok |  |
| Classic NES Series - Super Mario Bros. (USA, Europe) | FSME | Eeprom | ok |  |
| Classic NES Series - The Legend of Zelda (USA, Europe) | FZLE | Eeprom | ok |  |
| Classic NES Series - Xevious (USA, Europe) | FXVE | Eeprom | ok |  |
| Classic NES Series - Zelda II - The Adventure of Link (USA, Europe) | FLBE | Eeprom | ok |  |
| Cocoto - Kart Racer (Europe) (En,Fr,De,Es,It) | BC5P | None | ok |  |
| Cocoto - Platform Jumper (Europe) (En,Fr,De,Es,It) | BC8P | None | ok |  |
| Codename - Kids Next Door - Operation S.O.D.A. (USA) | BNDE | None | ok |  |
| Colin McRae Rally 2.0 (USA) (En,Fr,De) | ACME | Eeprom | ok |  |
| Columns Crown (USA) | ACGE | Eeprom | ok |  |
| Combat Choro Q - Advance Daisakusen (Japan) | AQCJ | Sram32K | ok |  |
| Comix Zone (Europe) (En,Fr,De,Es,It) | ACZP | Eeprom | ok |  |
| Contra Advance - The Alien Wars EX (USA) | AAWE | None | ok |  |
| Corvette (USA) (En,Fr,De,Es,It) | AVCE | Eeprom | ok |  |
| Crash Bandicoot - The Huge Adventure (USA) | ACQE | Eeprom | ok |  |
| Crash Bandicoot 2 - N-Tranced (USA) | AC8E | Eeprom | ok |  |
| Crash Bandicoot Purple - Ripto's Rampage (USA) (Rev 1) | BD4E | Eeprom | ok |  |
| Crash Nitro Kart (USA) | BCNE | Eeprom | ok |  |
| Crash of the Titans (USA) (En,Fr) | BQCE | Eeprom | ok |  |
| Crayon Shin-chan - Arashi o Yobu Cinemaland no Daibouken! (Japan) | BKCJ | Eeprom | ok |  |
| Crayon Shin-chan - Densetsu o Yobu Omake no Miyako Shockgaan! (Japan) | BC2J | Eeprom | ok |  |
| Crazy Chase (USA) | ACCE | Eeprom | ok |  |
| Crazy Frog Racer (Europe) (En,Fr,De,Nl) | BCRP | Eeprom | ok |  |
| Crazy Taxi - Catch a Ride (USA) | A3CE | Eeprom | ok |  |
| Creatures (Europe) (En,Es,It) | ACTY | Eeprom | ok |  |
| Croket! - Yume no Banker Survival! (Japan) | A6CJ | Sram32K | ok |  |
| Croket! 2 - Yami no Bank to Banqueen (Japan) | BK2J | Sram32K | ok |  |
| Croket! 3 - Granu Oukoku no Nazo (Japan) | B3KJ | Sram32K | ok |  |
| Croket! 4 - Bank no Mori no Mamorigami (Japan) | BK4J | Sram32K | ok |  |
| Croket! Great - Toki no Boukensha (Japan) | BK5J | Sram32K | ok |  |
| Crouching Tiger, Hidden Dragon (USA) (En,Fr,Es) | AQDE | None | ok |  |
| Cruis'n Velocity (USA, Europe) | ACFE | None | ok |  |
| Crushed Baseball (USA) | BCBE | Eeprom | ok |  |
| CT Special Forces (USA) (En,Fr,De,Es,It,Nl) | AC7E | None | ok |  |
| CT Special Forces 2 - Back in the Trenches (USA) (En,Fr,De,Es,It,Nl) | A9CE | None | ok |  |
| CT Special Forces 3 - Bioterror (Europe) (En,Fr,De,Es,It,Nl) | BC3P | None | ok |  |
| Cubix - Robots for Everyone - Clash 'N Bash (USA) | ACXE | Eeprom | ok |  |
| Curious George (USA) | B3JE | Eeprom | ok |  |
| Custom Robo GX (Japan) | ARJJ | Eeprom | ok |  |
| Cyberdrive Zoids - Kijuu no Senshi Hyuu (Japan) | AZ3J | Sram32K | ok |  |
| Dai-mahjong. (Japan) | AHMJ | Eeprom | ok |  |
| Daigassou! Band-Brothers - Request Selection (Japan) (DS Expansion Cartridge) | ZBBJ | None | not-a-game | not a game: DS slot-2 expansion data (no entry point); hardware would hang the same way |
| Daisenryaku for Game Boy Advance (Japan) | ADSJ | Flash64K | ok |  |
| Daisuki Teddy (Japan) | ATDJ | Sram32K | ok |  |
| Dan Doh!! Tobase Shouri no Smile Shot (Japan) | BDNJ | Sram32K | ok |  |
| Dan Doh!! Xi (Japan) | AXHJ | Sram32K | ok |  |
| Dancing Sword - Senkou (Japan) | A9SJ | Eeprom | ok |  |
| Danny Phantom - The Ultimate Enemy (USA) | BUEE | Eeprom | ok |  |
| Danny Phantom - Urban Jungle (USA) | BQYE | Eeprom | ok |  |
| Daredevil (USA, Europe) | AVLE | None | ok |  |
| Darius R (Japan) (En) | A2DJ | Eeprom | ok |  |
| Dark Arena (USA, Europe) | ADAE | None | ok |  |
| Dave Mirra Freestyle BMX 2 (USA) | AX2E | Eeprom | ok |  |
| Dave Mirra Freestyle BMX 3 (USA, Europe) | AB3E | Eeprom | ok |  |
| David Beckham Soccer (USA) (En,Es) | ABQE | Eeprom | ok |  |
| Davis Cup (USA) (En,Fr,De,Es,It) | AD6E | Eeprom | ok |  |
| Dead to Rights (USA) | BDEE | None | ok |  |
| Deal or No Deal (USA) | BZNE | None | ok |  |
| Defender (USA) | A2FE | Eeprom | ok |  |
| Defender of the Crown (USA) | ADHE | Eeprom | ok |  |
| DemiKids - Dark Version (USA) | AC5E | Sram32K | ok |  |
| DemiKids - Light Version (USA) | AL4E | Sram32K | ok |  |
| Demon Driver - Time to Burn Rubber! (USA) | A9AE | None | ok |  |
| Denki Blocks! (USA) (En,Es) | ADBE | Eeprom | ok |  |
| Densetsu no Stafy (Japan) | ASTJ | Sram32K | ok |  |
| Densetsu no Stafy 2 (Japan) | AVFJ | Sram32K | ok |  |
| Densetsu no Stafy 3 (Japan) (Rev 1) | B3DJ | Sram32K | ok |  |
| Derby Stallion Advance (Japan) | A8PJ | Flash64K | ok |  |
| Desert Strike Advance (USA) | ADIE | None | ok |  |
| Dexter's Laboratory - Chess Challenge (USA) | ADXE | None | ok |  |
| Dexter's Laboratory - Deesaster Strikes! (USA) (En,Fr,De,Es,It) (Rev 1) | ADLE | Eeprom | ok |  |
| Di Gi Charat - DigiCommunication (Japan) | A3OJ | Sram32K | ok |  |
| Diadroids World - Evil Teikoku no Yabou (Japan) | ADDJ | Eeprom | ok |  |
| DigiCommunication Nyo - Datou! Black Gemagema Dan (Japan) | BDKJ | Eeprom | ok |  |
| Digimon - Battle Spirit (USA) | A8SE | Eeprom | ok |  |
| Digimon - Battle Spirit 2 (USA) (En,Fr,De,Es,It) | BDSE | Eeprom | ok |  |
| Digimon Racing (USA) (En,Fr,De,Es,It) | BDGE | Eeprom | ok |  |
| Dinotopia - The Timestone Pirates (USA, Europe) (En,Fr,De,Es,It,Nl) | AD3E | Eeprom | ok |  |
| Disney Princess (USA, Europe) | AQPE | None | ok |  |
| Disney Princess - Royal Adventure (USA) | BQNE | Eeprom | ok |  |
| Disney Sports - Basketball (USA) | A2AE | Eeprom | ok |  |
| Disney Sports - Football (USA) | A3DE | Eeprom | ok |  |
| Disney Sports - Motocross (USA) | AOME | Eeprom | ok |  |
| Disney Sports - Skateboarding (USA) | A4DE | Eeprom | ok |  |
| Disney Sports - Snowboarding (USA) | A5DE | Eeprom | ok |  |
| Disney Sports - Soccer (USA) | A6DE | Eeprom | ok |  |
| Disney's Game + TV Episode - Lizzie McGuire 2 - Lizzie Diaries (USA) (En,Fr) | BL4E | Flash64K | ok |  |
| Disney's Party (USA, Europe) (En,Fr,De,Es,It) | BD8E | Eeprom | ok |  |
| DK - King of Swing (USA) | BBKE | Sram32K | ok |  |
| Dogz (USA) | B82E | Eeprom | ok |  |
| Dogz - Fashion (USA) | BFEE | Eeprom | ok |  |
| Dogz 2 (USA) (Rev 1) | BIME | Eeprom | ok |  |
| Dokapon - Monster Hunter (USA) | ADQE | Sram32K | ok |  |
| Dokidoki Cooking Series 1 - Komugi-chan no Happy Cake (Japan) | A56J | Eeprom | ok |  |
| Dokidoki Cooking Series 2 - Gourmet Kitchen - Suteki na Obentou (Japan) | A8OJ | Eeprom | ok |  |
| Dokodemo Taikyoku - Yakuman Advance (Japan) (Rev 1) | AYAJ | Sram32K | ok |  |
| Domo-kun no Fushigi Television (Japan) | ADOJ | Sram32K | ok |  |
| Don-chan Puzzle - Hanabi de Doon! Advance (Japan) | BDAJ | Eeprom | ok |  |
| Donald Duck Advance (USA) | ADKE | None | ok |  |
| Donkey Kong Country (USA) | A5NE | Eeprom | ok |  |
| Donkey Kong Country 2 (USA) | B2DE | Eeprom | ok |  |
| Donkey Kong Country 3 (USA) | BDQE | Eeprom | ok |  |
| Doom (USA, Europe) | ADME | Eeprom | ok |  |
| Doom II (USA) | A9DE | Eeprom | ok |  |
| Dora the Explorer - Dora's World Adventure! (USA) | BXPE | None | ok |  |
| Dora the Explorer - Super Spies (USA) | BERE | None | ok |  |
| Dora the Explorer - Super Star Adventures! (USA) | BDOE | None | ok |  |
| Dora the Explorer - The Search for the Pirate Pig's Treasure (USA) | AERE | None | ok |  |
| Doraemon - Dokodemo Walker (Japan) | ADPJ | Eeprom | ok |  |
| Doraemon - Midori no Wakusei Dokidoki Daikyuushutsu! (Japan) | ADRJ | Sram32K | ok |  |
| Double Dragon Advance (USA) | BDDE | None | ok |  |
| Doubutsu-jima no Chobigurumi (Japan) (Rev 1) | A8DJ | Flash64K | ok |  |
| Doubutsu-jima no Chobigurumi 2 - Tama-chan Monogatari (Japan) | BDCJ | Eeprom | ok |  |
| Downforce (Europe) (En,Fr,De,Es,It) | ADWE | None | ok |  |
| Dr. Muto (Europe) (En,Fr,De,Es,It) | A6TP | Eeprom | ok |  |
| Dr. Seuss' The Cat in the Hat (USA) | AQTE | Eeprom | ok |  |
| Dr. Sudoku (USA) | BUOE | Eeprom | ok |  |
| Dragon Ball - Advanced Adventure (USA) | BDVE | Eeprom | ok |  |
| Dragon Ball GT - Transformation (USA) | BT4E | Eeprom | ok |  |
| Dragon Ball Z - Buu's Fury (USA) | BG3E | Eeprom | ok |  |
| Dragon Ball Z - Collectible Card Game (USA) | ADZE | Sram32K | ok |  |
| Dragon Ball Z - Supersonic Warriors (USA) | AZJE | Eeprom | ok |  |
| Dragon Ball Z - Taiketsu (USA) | BDBE | Eeprom | ok |  |
| Dragon Ball Z - The Legacy of Goku (USA) | ALGE | Eeprom | ok |  |
| Dragon Ball Z - The Legacy of Goku II (USA) | ALFE | Eeprom | ok |  |
| Dragon Drive - World D Break (Japan) | A5GJ | Sram32K | ok |  |
| Dragon Quest Characters - Torneko no Daibouken 2 Advance - Fushigi no Dungeon (Japan) | AT2J | Flash64K | ok |  |
| Dragon Quest Characters - Torneko no Daibouken 3 Advance - Fushigi no Dungeon (Japan) | BD3J | Flash64K | ok |  |
| Dragon Quest Monsters - Caravan Heart (Japan) | A9HJ | Eeprom | ok |  |
| Dragon Tales - Dragon Adventures (USA) | BD9E | None | ok |  |
| Dragon's Rock (Europe) (En,Fr,De,Es,It) | BJDP | Eeprom | ok |  |
| Drake & Josh (USA) (En,Fr) | AJYE | None | ok |  |
| Drill Dozer (USA) | V49E | Sram32K | ok |  |
| Driv3r (USA) | B3RE | Eeprom | ok |  |
| Driven (USA) (En,Fr,De,Es,It) | ADVE | Eeprom | ok |  |
| Driver 2 Advance (USA) | ADUE | Eeprom | ok |  |
| Drome Racers (USA) | AOEE | Eeprom | ok |  |
| Droopy's Tennis Open (Europe) (En,Fr,De,Es,It,Nl) | AD7P | None | ok |  |
| Dual Blades (USA) | AB9E | Eeprom | ok |  |
| Duel Masters (Japan) | AA9J | Sram32K | ok |  |
| Duel Masters - Kaijudo Showdown (USA) | BD6E | Eeprom | ok |  |
| Duel Masters - Sempai Legends (USA) | AA9E | Eeprom | ok |  |
| Duel Masters - Shadow of the Code (USA) | BDUE | Eeprom | ok |  |
| Duel Masters 2 - Invincible Advance (Japan) | BD2J | Sram32K | ok |  |
| Duel Masters 2 - Kirifuda Shoubu Ver. (Japan) | BD5J | Sram32K | ok |  |
| Duel Masters 3 (Japan) (Rev 1) | BDUJ | Sram32K | ok |  |
| Duke Nukem Advance (USA) | AD9E | Eeprom | ok |  |
| Dungeons & Dragons - Eye of the Beholder (USA) | AD4E | Sram32K | ok |  |
| Dynasty Warriors Advance (USA) | B36E | Sram32K | ok |  |
| E.T. - The Extra-Terrestrial (USA) | AETE | None | ok |  |
| Earthworm Jim (USA, Europe) | AEJE | None | ok |  |
| Earthworm Jim 2 (USA) | AJ4E | None | ok |  |
| Ecks vs Sever (USA) (En,Fr,De,Es,It) | AESE | None | ok |  |
| Ed, Edd n Eddy - Jawbreakers! (USA) (Rev 1) | AE3E | Eeprom | ok |  |
| Ed, Edd n Eddy - The Mis-Edventures (USA) (En,Fr) | BEDE | Eeprom | ok |  |
| Egg Mania (USA) (En,Fr,Es) | AEME | None | ok |  |
| Elemix! (Japan) | AEKJ | Eeprom | ok |  |
| Elevator Action - Old & New (Japan) | ANWJ | Eeprom | ok |  |
| Elf - The Movie (USA) (En,Fr,De,Es,It) | BELE | None | ok |  |
| Elf Bowling 1 & 2 (USA) | BEBE | None | ok |  |
| Enchanted - Once Upon Andalasia (USA) (En,Fr) | BZRE | Eeprom | ok |  |
| Eragon (USA) | BENE | Eeprom | ok |  |
| Erementar Gerad - Tozasareshi Uta (Japan) | BEJJ | Eeprom | ok |  |
| ESPN Final Round Golf 2002 (USA) | AGRE | Flash64K | ok |  |
| ESPN Great Outdoor Games - Bass 2002 (USA) | AMGE | Sram32K | ok |  |
| ESPN International Winter Sports 2002 (USA) | AWIE | Eeprom | ok |  |
| ESPN Winter X-Games Snowboarding 2002 (USA) | AWXE | Eeprom | ok |  |
| ESPN X-Games Skateboarding (USA) | AXSE | Eeprom | ok |  |
| European Super League (Europe) (En,Fr,De,Es,It) | AELP | None | ok |  |
| everGirl (USA) | BEVE | Eeprom | ok |  |
| EX Monopoly (Japan) (Rev 1) | AMOJ | Sram32K | ok |  |
| Extreme Ghostbusters - Code Ecto-1 (USA) | AEGE | None | ok |  |
| Extreme Skate Adventure (USA) (Rev 1) | BESE | Eeprom | ok |  |
| Eyeshield 21 - DevilBats DevilDays (Japan) | BE4J | Sram32K | ok |  |
| EZ-Talk - Shokyuu Hen 1 (Japan) | A22J | Sram32K | ok |  |
| EZ-Talk - Shokyuu Hen 2 (Japan) | A23J | Sram32K | ok |  |
| EZ-Talk - Shokyuu Hen 3 (Japan) | A24J | Sram32K | ok |  |
| EZ-Talk - Shokyuu Hen 4 (Japan) | A25J | Sram32K | ok |  |
| EZ-Talk - Shokyuu Hen 5 (Japan) | A26J | Sram32K | ok |  |
| EZ-Talk - Shokyuu Hen 6 (Japan) | A27J | Sram32K | ok |  |
| F-14 Tomcat (USA, Europe) | AFTE | None | ok |  |
| F-Zero - Climax (Japan) | BFTJ | Flash128K | ok |  |
| F-Zero - GP Legend (USA) | BFZE | Sram32K | ok |  |
| F-Zero - Maximum Velocity (USA, Europe) | AFZE | Sram32K | ok |  |
| F1 2002 (USA, Europe) | AF8E | Eeprom | ok |  |
| F24 Stealth Fighter (USA) | BYAE | None | ok |  |
| Fairly OddParents!, The - Breakin' da Rules (USA) | AF6E | None | ok |  |
| Fairly OddParents!, The - Clash with the Anti-World (USA) | BFOE | None | ok |  |
| Fairly OddParents!, The - Enter the Cleft (USA) | AFVE | None | ok |  |
| Fairly OddParents!, The - Shadow Showdown (USA) | BF2E | None | ok |  |
| Famicom Mini 08 - Mappy (Japan) (En) | FMPJ | Eeprom | ok |  |
| Famicom Mini 10 - Star Soldier (Japan) (En) | FSOJ | Eeprom | ok |  |
| Famicom Mini 11 - Mario Bros. (Japan) | FMBJ | Eeprom | ok |  |
| Famicom Mini 12 - Clu Clu Land (Japan) | FCLJ | Eeprom | ok |  |
| Famicom Mini 13 - Balloon Fight (Japan) | FBFJ | Eeprom | ok |  |
| Famicom Mini 14 - Wrecking Crew (Japan) | FWCJ | Eeprom | ok |  |
| Famicom Mini 16 - Dig Dug (Japan) | FDDJ | Eeprom | ok |  |
| Famicom Mini 17 - Takahashi Meijin no Bouken-jima (Japan) | FTBJ | Eeprom | ok |  |
| Famicom Mini 18 - Makaimura (Japan) | FMKJ | Eeprom | ok |  |
| Famicom Mini 19 - Twin Bee (Japan) | FTWJ | Eeprom | ok |  |
| Famicom Mini 20 - Ganbare Goemon! - Karakuri Douchuu (Japan) | FGGJ | Eeprom | ok |  |
| Famicom Mini 21 - Super Mario Bros. 2 (Japan) | FM2J | Eeprom | ok |  |
| Famicom Mini 22 - Nazo no Murasame Jou (Japan) | FNMJ | Eeprom | ok |  |
| Famicom Mini 24 - Hikari Shinwa - Palthena no Kagami (Japan) | FPTJ | Eeprom | ok |  |
| Famicom Mini 26 - Famicom Mukashibanashi - Shin Onigashima - Zen, Kouhen (Japan) | FFMJ | Eeprom | ok |  |
| Famicom Mini 27 - Famicom Tantei Club - Kieta Koukeisha - Zen, Kouhen (Japan) | FTKJ | Eeprom | ok |  |
| Famicom Mini 28 - Famicom Tantei Club Part II - Ushiro ni Tatsu Shoujo - Zen, Kouhen (Japan) | FTUJ | Eeprom | ok |  |
| Famicom Mini 30 - SD Gundam World - Gachapon Senshi Scramble Wars (Japan) | FSDJ | Eeprom | ok |  |
| Family Feud (USA) | B2FE | None | ok |  |
| Family Tennis Advance (Japan) | AATJ | Eeprom | ok |  |
| Famista Advance (Japan) | AN7J | Eeprom | ok |  |
| Fancy Pocket (Japan) | AJEJ | Sram32K | ok |  |
| Fantastic 4 (USA) | BF4E | Eeprom | ok |  |
| Fantastic 4 - Flame On (USA) | BH4E | Eeprom | ok |  |
| Fantastic Children (Japan) | BFCJ | Eeprom | ok |  |
| Fantastic Maerchen - Cake-ya-san Monogatari + Doubutsu Chara Navi Uranai - Kosei Shinri Gaku (Japan) | AAXJ | Eeprom | ok |  |
| Fear Factor - Unleashed (USA) | BFUE | Eeprom | ok |  |
| Field of Nine - Digital Edition 2001 (Japan) | AF9J | Sram32K | ok |  |
| FIFA Soccer 06 (USA, Europe) (En,Fr,De,Es,It,Nl) | BF6E | Eeprom | ok |  |
| FIFA Soccer 07 (USA, Europe) (En,Fr,De,Es) | B7FE | Eeprom | ok |  |
| FIFA Soccer 2003 (USA, Europe) (En,Fr,De,Es,It) | AFJE | Eeprom | ok |  |
| FIFA Soccer 2004 (USA, Europe) (En,Fr,De,Es,It) | BFIE | Eeprom | ok |  |
| FIFA Soccer 2005 (USA, Europe) (En,Fr,De,Es,It,Nl) | BF5E | Eeprom | ok |  |
| FightBox (Europe) | BOXP | Eeprom | ok |  |
| FILA Decathlon (Europe) (En,Fr,De,Es,It,Sv) | AFLP | Eeprom | ok |  |
| Final Fantasy I & II - Dawn of Souls (USA) | BFFE | Sram32K | ok |  |
| Final Fantasy IV Advance (USA) | BZ4E | Sram32K | ok |  |
| Final Fantasy Tactics Advance (USA) | AFXE | Flash64K | ok |  |
| Final Fantasy V Advance (USA) | BZ5E | Sram32K | ok |  |
| Final Fantasy VI Advance (USA) | BZ6E | Sram32K | ok |  |
| Final Fight One (USA) | AFFE | Eeprom | ok |  |
| Finding Nemo (USA, Europe) | AZIE | None | ok |  |
| Finding Nemo - The Continuing Adventures (USA, Europe) | BZIE | None | ok |  |
| Fire Emblem (USA, Australia) | AE7E | Sram32K | ok |  |
| Fire Emblem - Fuuin no Tsurugi (Japan) | AFEJ | Sram32K | ok |  |
| Fire Emblem - The Sacred Stones (USA, Australia) | BE8E | Sram32K | ok |  |
| Fire Pro Wrestling (USA, Europe) | AFPE | Flash64K | ok |  |
| Fire Pro Wrestling 2 (USA) | AFYE | Flash64K | ok |  |
| Flintstones, The - Big Trouble in Bedrock (USA) | AFSE | None | ok |  |
| Flushed Away (USA) | BLHE | None | ok |  |
| Ford Racing 3 (USA) | BF3E | None | ok |  |
| Formation Soccer 2002 (Japan) | AFMJ | Sram32K | ok |  |
| Fortress (USA, Europe) | AFOE | None | ok |  |
| Foster's Home for Imaginary Friends (USA) | BFYE | Eeprom | ok |  |
| Franklin the Turtle (USA) (En,Fr,De,Es,It,Sv,No,Da,Fi) | BFKE | Eeprom | ok |  |
| Franklin's Great Adventures (USA) (En,Fr,Es) | BFLE | Eeprom | ok |  |
| Freekstyle (USA) | BFSE | None | ok |  |
| Frogger Advance - The Great Quest (USA) | AFQE | None | ok |  |
| Frogger's Adventures - Temple of the Frog (USA) (En,Fr,De,Es,It) | AFRE | Eeprom | ok |  |
| Frogger's Adventures 2 - The Lost Wand (USA) (En,Es) | AFBE | Eeprom | ok |  |
| Frogger's Journey - The Forgotten Relic (USA) | BFJE | Eeprom | ok |  |
| From TV Animation One Piece - Mezase! King of Berry (Japan) | AUSJ | Flash64K | ok |  |
| From TV Animation One Piece - Nanatsu-jima no Daihihou (Japan) | AO7J | Sram32K | ok |  |
| Fruits Mura no Doubutsu-tachi (Japan) (Rev 2) | BFDJ | Eeprom | ok |  |
| Fushigi no Kuni no Alice (Japan) | AF4J | Eeprom | ok |  |
| Fushigi no Kuni no Angelique (Japan) | AFAJ | Sram32K | ok |  |
| Futari wa Pretty Cure - Arienaai! Yume no Sono wa Daimeikyuu (Japan) | BFPJ | Eeprom | ok |  |
| Futari wa Pretty Cure Max Heart - Maji Maji! Fight de IN Janai (Japan) | BFMJ | Eeprom | ok |  |
| Gachasute! Dino Device - Blue (Japan) | ABIJ | Flash64K | ok |  |
| Gachasute! Dino Device - Red (Japan) | AAIJ | Flash64K | ok |  |
| Gachasute! Dino Device 2 - Dragon (Japan) | A4XJ | Eeprom | ok |  |
| Gachasute! Dino Device 2 - Phoenix (Japan) | A4WJ | Eeprom | ok |  |
| Gachinko Pro Yakyuu (Japan) | ANYJ | Sram32K | ok |  |
| Gadget Racers (USA) | AQAE | Sram32K | ok |  |
| Gakkou no Kaidan - Hyakuyoubako no Fuuin (Japan) | BGHJ | Eeprom | ok |  |
| Gakkou o Tsukurou!! Advance (Japan) | AYSJ | Eeprom | ok |  |
| Gakuen Alice - Dokidoki Fushigi Taiken (Japan) (Rev 1) | BASJ | Sram32K | ok |  |
| Gakuen Senki Muryou (Japan) | BGSJ | Eeprom | ok |  |
| Galaxy Angel Game Boy Advance - Moridakusan Tenshi no Full-Course - Okawari Jiyuu (Japan) | AGZJ | Eeprom | ok |  |
| Galidor - Defenders of the Outer Dimension (USA) (En,Fr,De,Es,It,Nl,Sv,Da) | AG8E | Eeprom | ok |  |
| Gambler Densetsu Tetsuya - Yomigaeru Densetsu (Japan) | ATYJ | Eeprom | ok |  |
| Game & Watch Gallery 4 (USA) | AQWE | Sram32K | ok |  |
| Game Boy Wars Advance 1+2 (Japan) | BGWJ | Flash128K | ok |  |
| Games Explosion! (USA) | BG7E | None | ok |  |
| Ganbare! Dodge Fighters (Japan) | BG8J | Eeprom | ok |  |
| Garfield - The Search for Pooky (USA) (En,Fr,De,Es,It) | BGOE | None | ok |  |
| Garfield and His Nine Lives (USA) (En,Fr,Es) | BG9E | Eeprom | ok |  |
| Gauntlet - Dark Legacy (USA) | AYGE | Eeprom | ok |  |
| Gegege no Kitarou - Kikiippatsu! Youkai Rettou (Japan) | BGKJ | Sram32K | ok |  |
| Gekido Advance - Kintaro's Revenge (USA) | AGEE | None | ok |  |
| Gekitou Densetsu Noah - Dream Management (Japan) | ANNJ | Eeprom | ok |  |
| Gem Smashers (USA) | AZSE | Eeprom | ok |  |
| Genseishin Justirisers - Souchaku! Hoshi no Senshi-tachi (Japan) | BGJJ | Flash64K | ok |  |
| Gensou Maden Saiyuuki - Hangyaku no Toushin-taishi (Japan) | BGMJ | Eeprom | ok |  |
| Gensou Suikoden - Card Stories (Japan) | AGKJ | None | ok |  |
| Get Ride! Amdriver - Senkou no Hero Tanjou! (Japan) | BGIJ | Flash64K | ok |  |
| Get Ride! Amdriver - Shutsugeki! Battle Party (Japan) | BGPJ | Sram32K | ok |  |
| Get! - Boku no Mushi Tsukamaete (Japan) | BGBJ | Eeprom | ok |  |
| GetBackers Dakkanya - Jagan Fuuin! (Japan) | BGFJ | Sram32K | ok |  |
| GetBackers Dakkanya - Jigoku no Scaramouche (Japan) | AGBJ | Flash64K | ok |  |
| GetBackers Dakkanya - Metropolis Dakkan Sakusen! (Japan) | A8GJ | Flash64K | ok |  |
| Ghost Rider (USA, Europe) (En,Fr,De,Es,It,Nl) | BR8E | Eeprom | ok |  |
| Ghost Trap (Japan) | AGVJ | Eeprom | ok |  |
| Global Star - Sudoku Fever (USA) | B3ZE | Eeprom | ok |  |
| Glucoboy (Australia) | BJGE | Eeprom | ok |  |
| Go! Go! Beckham! - Adventure on Soccer Island (Europe) (En,Fr,De,Es,It) | AGQP | Eeprom | ok |  |
| Godzilla - Domination! (USA) | AG4E | None | ok |  |
| Goemon - New Age Shutsudou! (Japan) | AGNJ | Eeprom | ok |  |
| Golden Nugget Casino (USA, Europe) | BGGE | None | ok |  |
| Golden Sun (USA, Europe) | AGSE | Flash64K | ok |  |
| Golden Sun - The Lost Age (USA, Europe) | AGFE | Flash64K | ok |  |
| Gradius Galaxies (USA) | AGAE | Eeprom | ok |  |
| Grand Theft Auto (USA) | BGTE | Eeprom | ok |  |
| Green Eggs and Ham by Dr. Seuss (USA) | BUSE | Eeprom | ok |  |
| Greg Hastings' Tournament Paintball Max'd (USA) | BGQE | Eeprom | ok |  |
| Gremlins - Stripe vs Gizmo (USA) | AGGE | None | ok |  |
| Grim Adventures of Billy & Mandy, The (USA) | BIEE | Eeprom | ok |  |
| Groove Adventure Rave - Hikari to Yami no Daikessen (Japan) | ARVJ | Sram32K | ok |  |
| GT Advance - Championship Racing (USA, Europe) | ACAE | None | ok |  |
| GT Advance 2 - Rally Racing (USA) | AGWE | Eeprom | ok |  |
| GT Advance 3 - Pro Concept Racing (USA) | A2GE | Eeprom | ok |  |
| GT Racers (Europe) (En,Fr,De,Es,It) | BJAP | None | ok |  |
| Guilty Gear X - Advance Edition (USA) | AGXE | Flash64K | ok |  |
| Gumby vs. the Astrobots (USA) | BGVE | Eeprom | ok |  |
| Gunstar Super Heroes (USA) | BHGE | Eeprom | ok |  |
| Guranbo (Japan) | AIBJ | Sram32K | ok |  |
| Guru Logic Champ (Japan) | AGCJ | Flash64K | ok |  |
| Gyakuten Saiban (Japan) (Rev 1) | ASBJ | Sram32K | ok |  |
| Gyakuten Saiban 2 (Japan) | A3GJ | Sram32K | ok |  |
| Gyakuten Saiban 3 (Japan) | A3JJ | Sram32K | ok |  |
| Hachiemon (Japan) | A8EJ | Eeprom | ok |  |
| Hagane no Renkinjutsushi - Meisou no Rondo (Japan) | BHRJ | Eeprom | ok |  |
| Hagane no Renkinjutsushi - Omoide no Sonata (Japan) | BH2J | Eeprom | ok |  |
| Hajime no Ippo - The Fighting! (Japan) | A2HJ | Eeprom | ok |  |
| Hamepane - Tokyo Mew Mew (Japan) | AM7J | Sram32K | ok |  |
| Hamster Club 4 - Shigetchi Daidassou (Japan) | A4KJ | Sram32K | ok |  |
| Hamster Monogatari 2 GBA (Japan) | AHBJ | Sram32K | ok |  |
| Hamster Monogatari 3 GBA (Japan) | A83J | Eeprom | ok |  |
| Hamster Monogatari 3EX 4 Special (Japan) | BHSJ | Eeprom | ok |  |
| Hamster Monogatari Collection (Japan) | BHCJ | Eeprom | ok |  |
| Hamster Paradise - Pure Heart (Japan) | A82J | Eeprom | ok |  |
| Hamster Paradise Advanchu (Japan) | AHAJ | Eeprom | ok |  |
| Hamtaro - Ham-Ham Games (Japan, USA) (En,Ja) | B85A | Sram32K | ok |  |
| Hamtaro - Ham-Ham Heartbreak (USA) | AH3E | Sram32K | ok |  |
| Hamtaro - Rainbow Rescue (Europe) (En,Fr,De,Es,It) | A84P | Sram32K | ok |  |
| Hanabi Hyakkei Advance (Japan) | BHAJ | Flash64K | ok |  |
| Hanafuda Trump Mahjong - Depachika Wayouchuu (Japan) | ADYJ | Eeprom | ok |  |
| Happy Feet (USA) (En,Fr) | BH3E | Eeprom | ok |  |
| Hardcore Pinball (USA, Europe) | AH6E | Eeprom | ok |  |
| Hardcore Pool (Europe) (En,De,Es,It) | BHOP | None | ok |  |
| Harlem Globetrotters - World Tour (USA) | BHNE | None | ok |  |
| Harobots - Robo Hero Battling!! (Japan) | AHQJ | Sram32K | ok |  |
| Harry Potter - Quidditch World Cup (USA, Europe) (En,Fr,De,Es,It,Nl,Da) | BHPE | Eeprom | ok |  |
| Harry Potter and the Chamber of Secrets (USA, Europe) (En,Fr,De,Es,It,Nl,Pt,Sv,No,Da) | A7HE | Eeprom | ok |  |
| Harry Potter and the Goblet of Fire (USA, Europe) (En,Fr,De,Es,It,Nl,Da) | BH8E | Eeprom | ok |  |
| Harry Potter and the Order of the Phoenix (USA, Europe) (En,Fr,De,Es,It,Nl,Da) | BJXE | Eeprom | ok |  |
| Harry Potter and the Prisoner of Azkaban (USA, Europe) (En,Fr,De,Es,It,Nl,Da) | BHTE | Eeprom | ok |  |
| Harry Potter and the Sorcerer's Stone (USA, Europe) (En,Fr,De,Es,It,Nl,Pt,Sv,No,Da) | AHRE | Eeprom | ok |  |
| Harvest Moon - Friends of Mineral Town (USA) | A4NE | Sram32K | ok |  |
| Harvest Moon - More Friends of Mineral Town (USA) | BFGE | Sram32K | ok |  |
| Hatena Satena (Japan) | AHSJ | Sram32K | ok |  |
| Heidi - The Game (Europe) (En,Fr,De,Es,It) | BHJP | Eeprom | ok |  |
| Hello Kitty - Happy Party Pals (USA) | B86E | None | ok |  |
| Hello Kitty Collection - Miracle Fashion Maker (Japan) | AKTJ | Eeprom | black | OPEN: the game's first malloc walks an uninitialised heap through BIOS open-bus values; mGBA escapes the loop, we do not (see PROGRESS.md) |
| Hello! Idol Debut - Kids Idol Ikusei Game (Japan) (Rev 1) | BHDJ | Eeprom | ok |  |
| Herbie - Fully Loaded (USA) | B8FE | Eeprom | ok |  |
| Hey Arnold! - The Movie (USA) | AAEE | None | ok |  |
| Hi Hi Puffy AmiYumi - Kaznapped! (USA) | BHHE | Eeprom | ok |  |
| Higanbana (Japan) (Rev 1) | AHZJ | Eeprom | ok |  |
| High Heat Major League Baseball 2002 (USA, Europe) | ASSE | Eeprom | ok |  |
| High Heat Major League Baseball 2003 (USA) | AHHE | Eeprom | ok |  |
| High School Musical - Livin' the Dream (USA) | BJ2E | Eeprom | ok |  |
| Hikaru no Go (Japan) (Rev 1) | AHKJ | Sram32K | ok |  |
| Hikaru no Go 2 (Japan) | AKEJ | Sram32K | ok |  |
| Hime Kishi Monogatari - Princess Blue (Japan) | A3HJ | Sram32K | ok |  |
| Hobbit, The - The Prelude to the Lord of the Rings (USA) | AH9E | Eeprom | ok |  |
| Home on the Range (USA) (En,Fr) | BHME | None | ok |  |
| Horsez (USA) | BHUE | Eeprom | ok |  |
| Hot Potato! (USA) | AHPE | Eeprom | ok |  |
| Hot Wheels - All Out (USA) | BHXE | None | ok |  |
| Hot Wheels - Burnin' Rubber (USA) | AHWE | Eeprom | ok |  |
| Hot Wheels - Stunt Track Challenge (USA, Europe) | BHEE | None | ok |  |
| Hot Wheels - Velocity X (USA) | AH8E | None | ok |  |
| Hot Wheels - World Race (USA) | BHWE | None | ok |  |
| Hudson Best Collection Vol. 1 - Bomberman Collection (Japan) | B7IJ | None | ok |  |
| Hudson Best Collection Vol. 2 - Lode Runner Collection (Japan) | B72J | None | ok |  |
| Hudson Best Collection Vol. 3 - Action Collection (Japan) | B73J | None | ok |  |
| Hudson Best Collection Vol. 4 - Nazotoki Collection (Japan) | B74J | None | ok |  |
| Hudson Best Collection Vol. 5 - Shooting Collection (Japan) | B75J | None | ok |  |
| Hudson Best Collection Vol. 6 - Bouken-jima Collection (Japan) | B76J | None | ok |  |
| Hugo - Bukkazoom! (Europe) (En,Fr,De,Es,It,Nl,Pt,Sv,No,Da,Fi,Pl) | AZHP | None | ok |  |
| Hugo - The Evil Mirror (USA) (En,Fr,Es) | AHJE | Eeprom | ok |  |
| Hunter X Hunter - Minna Tomodachi Daisakusen!! (Japan) (Rev 1) | A8NJ | Sram32K | ok |  |
| I Spy Challenger! (USA) | A4CE | Eeprom | ok |  |
| Ice Age (USA) (En,Fr,Es) | AIAE | None | ok |  |
| Ice Age 2 - The Meltdown (USA) | BIAE | Eeprom | ok |  |
| Ice Nine (USA, Europe) (En,Fr,De,Es,It) | AR3E | Eeprom | ok |  |
| Ignition Collection - Volume 1 (Europe) | BIVP | None | ok |  |
| IK+ (USA) | A3KE | None | ok |  |
| Incredible Hulk, The (USA) | AHLE | Eeprom | ok |  |
| Incredibles, The (USA, Europe) | BICE | None | ok |  |
| Incredibles, The - Rise of the Underminer (USA, Europe) | BIQE | None | ok |  |
| Initial D - Another Stage (Japan) | AINJ | Sram32K | ok |  |
| Inspector Gadget - Advance Mission (USA) | AIGE | None | ok |  |
| Inspector Gadget Racing (Europe) (En,Fr,De,Es,It,Nl) | AIRP | None | ok |  |
| International Karate Advanced (Europe) | AIKP | None | ok |  |
| International Superstar Soccer (Europe) | AISP | Eeprom | ok |  |
| International Superstar Soccer Advance (Europe) | AY2P | Eeprom | ok |  |
| Inukko Club (Japan) | AI9J | Sram32K | ok |  |
| Inuyasha - Naraku no Wana! Mayoi no Mori no Shoutaijou (Japan) | AIYJ | Sram32K | ok |  |
| Invader (Europe) | AIVP | None | ok |  |
| Invincible Iron Man, The (USA, Europe) | AIOE | Eeprom | ok |  |
| Iridion 3D (USA, Europe) | AI3E | None | ok |  |
| Iridion II (USA) | AI2E | None | ok |  |
| Iron Kid (Korea) | BIRK | Eeprom | ok |  |
| Island Xtreme Stunts (USA, Europe) (En,Fr,De,Es,It,Nl,Sv,Da) | AXTE | Eeprom | ok |  |
| Isseki Hatchou - Kore 1ppon de 8shurui! (Japan) | AIEJ | Sram32K | ok |  |
| It's Mr. Pants (USA, Europe) | BPIE | Eeprom | ok |  |
| J.League Pocket (Japan) (Rev 1) | AJPJ | Sram32K | ok |  |
| J.League Pocket 2 (Japan) | AJ2J | Sram32K | ok |  |
| J.League Pro Soccer Club o Tsukurou! Advance (Japan) | AC2J | Sram32K | ok |  |
| J.League Winning Eleven Advance 2002 (Japan) | A2JJ | Eeprom | ok |  |
| Jackie Chan Adventures - Legend of the Dark Hand (USA, Europe) | AJCE | Eeprom | ok |  |
| Jajamaru Jr. Denshouki - Jalecolle mo Arisourou (Japan) | BNJJ | Eeprom | ok |  |
| James Pond - Codename Robocod (USA) (En,Fr,Es,Pt) | AJDE | None | ok |  |
| Jazz Jackrabbit (USA, Europe) | AJJE | Eeprom | ok |  |
| Jet Grind Radio (USA) | AJRE | Eeprom | ok |  |
| Jimmy Neutron - Boy Genius (USA) | AJNE | None | ok |  |
| Jinsei Game Advance (Japan) | AZGJ | Sram32K | ok |  |
| Jissen Pachi-Slot Hisshouhou! - Juuou Advance (Japan) | AJUJ | Flash64K | ok |  |
| Jonny Moseley Mad Trix (USA, Europe) (En,Fr,De,Es,It) | AJME | Eeprom | ok |  |
| Juka and the Monophonic Menace (USA) (En,Fr,Es) | BJKE | Eeprom | ok |  |
| Jungle Book, The (USA) (En,Fr,De,Es,It,Nl) | AJFE | None | ok |  |
| Jurassic Park III - Island Attack (USA) | AJQE | Eeprom | ok |  |
| Jurassic Park III - Park Builder (USA) | AJ3E | Sram32K | ok |  |
| Jurassic Park III - The DNA Factor (USA) | ADNE | Eeprom | ok |  |
| Jurassic Park Institute Tour - Dinosaur Rescue (Japan) | AJ8J | Eeprom | ok |  |
| Justice League - Chronicles (USA) | BJLE | Eeprom | ok |  |
| Justice League - Injustice for All (USA) | AJLE | Eeprom | ok |  |
| Justice League Heroes - The Flash (USA) | BJHE | Eeprom | ok |  |
| K-1 Pocket Grand Prix (Japan) | AKVJ | Eeprom | ok |  |
| K-1 Pocket Grand Prix 2 (Japan) | A2OJ | Eeprom | ok |  |
| Kaeru B Back (Japan) | AKDJ | Sram32K | ok |  |
| Kaiketsu Zorori to Mahou no Yuuenchi - Ohimesama o Sukue! (Japan) | BKOJ | Eeprom | ok |  |
| Kamaitachi no Yoru Advance (Japan) | AKZJ | Flash64K | ok |  |
| Kami no Kijutsu - Illusion of the Evil Eyes (Japan) | AG2J | Sram32K | ok |  |
| KAO the Kangaroo (USA) (En,Fr,De,Es,It,Nl) | AKKE | None | ok |  |
| Kappa no Kai-kata - Kaatan Daibouken! (Japan) | BK8J | Sram32K | ok |  |
| Karnaaj Rally (USA, Europe) | AYKE | Flash64K | ok |  |
| Kawa no Nushi Tsuri 3 & 4 (Japan) | BN4J | Sram32K | ok |  |
| Kawa no Nushi Tsuri 5 - Fushigi no Mori kara (Japan) | AN5J | Eeprom | ok |  |
| Kawaii Pet Game Gallery (Japan) | BKGJ | Eeprom | ok |  |
| Kawaii Pet Game Gallery 2 (Japan) | BKPJ | Eeprom | ok |  |
| Kawaii Pet Shop Monogatari 3 (Japan) | A63J | Eeprom | ok |  |
| Keitai Denjuu Telefang 2 - Power (Japan) | ATPJ | Flash64K | ok |  |
| Keitai Denjuu Telefang 2 - Speed (Japan) | ATSJ | Flash64K | ok |  |
| Kelly Slater's Pro Surfer (USA, Europe) | AS3E | Eeprom | ok |  |
| Keroro Gunsou - Taiketsu! Gekisou Keronprix Daisakusen de Arimasu!! (Japan) | BKJJ | Sram32K | ok |  |
| Kessaku Sen! - Ganbare Goemon 1, 2 - Yuki Hime to Magginesu (Japan) | BG2J | Eeprom | ok |  |
| Kid Paddle (Europe) (Fr,Nl) | BYLP | Eeprom | ok |  |
| Kid's Cards (USA) | BCXE | None | ok |  |
| Kidou Gekidan Haro Ichiza - Haro no Puyo Puyo (Japan) | BH6J | Eeprom | ok |  |
| Kidou Senshi Gundam - Seed Destiny (Japan) | B42J | Eeprom | ok |  |
| Kidou Senshi Gundam Seed - Tomo to Kimi to Koko de. (Japan) | BGNJ | Eeprom | ok |  |
| Kidou Tenshi Angelic Layer - Misaki to Yume no Tenshi-tachi (Japan) | AALJ | Eeprom | ok |  |
| Kill Switch (USA) | BKHE | Eeprom | ok |  |
| Killer 3D Pool (USA) | B3LE | None | ok |  |
| Kim Possible - Revenge of Monkey Fist (USA) | AEYE | None | ok |  |
| Kim Possible 2 - Drakken's Demise (USA) (En,Fr) | BKME | Eeprom | ok |  |
| Kim Possible 3 - Team Possible (USA) (En,Fr) | BQPE | Eeprom | ok |  |
| King of Fighters EX 2, The - Howling Blood (USA) | AEXE | Eeprom | ok |  |
| King of Fighters EX, The - NeoBlood (USA) (Rev 1) | AKOE | Eeprom | ok |  |
| Kingdom Hearts - Chain of Memories (USA) | B8CE | Sram32K | ok |  |
| Kinniku Banzuke - Kimero! Kiseki no Kanzen Seiha (Japan) | AK5J | Sram32K | ok |  |
| Kinniku Banzuke - Kongou-kun no Daibouken! (Japan) | AK4J | Sram32K | ok |  |
| Kirby & The Amazing Mirror (USA) | B8KE | Sram32K | ok |  |
| Kirby - Nightmare in Dream Land (USA) | A7KE | Sram32K | ok |  |
| Kisekko Gurumii - Chesty to Nuigurumi-tachi no Mahou no Bouken (Japan) | A2VJ | Eeprom | ok |  |
| Kiss x Kiss - Seirei Gakuen (Japan) | B2KJ | Eeprom | ok |  |
| Kiwame Mahjong Deluxe - Mirai Senshi 21 (Japan) | AKMJ | Sram32K | waiting | menu waiting for input (name entry) |
| Klonoa - Empire of Dreams (USA) | AKLE | Eeprom | ok |  |
| Klonoa 2 - Dream Champ Tournament (USA) | AN6E | Eeprom | ok |  |
| Klonoa Heroes - Densetsu no Star Medal (Japan) | AK7J | Eeprom | ok |  |
| Knights' Kingdom (USA) | BKNE | None | ok |  |
| Koala Brothers - Outback Adventures (USA) (En,Fr,De,Es,It,Nl,Pt,Da) | BAKE | None | ok |  |
| Koinu to Issho - Aijou Monogatari (Japan) | BDIJ | Eeprom | ok |  |
| Koinu to Issho 2 (Japan) | BI2J | Eeprom | ok |  |
| Koinu-chan no Hajimete no Osanpo - Koinu no Kokoro Ikusei Game (Japan) | BISJ | Eeprom | ok |  |
| Konami Collector's Series - Arcade Advanced (USA) | AKCE | None | ok |  |
| Konami Krazy Racers (USA) | AKWE | Sram32K | ok |  |
| Konchuu Monster - Battle Master (Japan) | BQBJ | Eeprom | ok |  |
| Konchuu Monster - Battle Stadium (Japan) | BQSJ | Eeprom | ok |  |
| Konchuu no Mori no Daibouken - Fushigi na Sekai no Juunin-tachi (Japan) | BQKJ | Eeprom | ok |  |
| Kong - King of Atlantis (USA) | BK7E | None | ok |  |
| Kong - The 8th Wonder of the World (USA) (En,Fr,Es) | BKQE | Eeprom | ok |  |
| Kong - The Animated Series (USA) (En,Fr,De,Es,It,Nl) | AKQE | None | ok |  |
| Konjiki no Gashbell!! - Makai no Bookmark (Japan) | BKBJ | Sram32K | ok |  |
| Konjiki no Gashbell!! - Unare! Yuujou no Zakeru 2 (Japan) | BGYJ | Eeprom | ok |  |
| Konjiki no Gashbell!! The Card Battle for GBA (Japan) | BKEJ | Eeprom | ok |  |
| Konjiki no Gashbell!! Yuujou no Zakeru - Dream Tag Tournament (Japan) | BUDJ | Eeprom | ok |  |
| Korokoro Puzzle - Happy Panecchu! (Japan) | KHPJ | Eeprom | ok |  |
| Kotoba no Puzzle - Mojipittan Advance (Japan) | A8MJ | Sram32K | ok |  |
| Kouchuu Ouja Mushiking - Greatest Champion e no Michi (Japan) | BK6J | Sram32K | ok |  |
| Koukou Juken Advance Series Eigo Koubun Hen - 26 Units Shuuroku (Japan) | A54J | Sram32K | ok |  |
| Koukou Juken Advance Series Eijukugo Hen - 650 Phrases Shuuroku (Japan) | A53J | Sram32K | ok |  |
| Koukou Juken Advance Series Eitango Hen - 2000 Words Shuuroku (Japan) | A52J | Sram32K | ok |  |
| Kunio-kun Nekketsu Collection 1 (Japan) | B9AJ | None | ok |  |
| Kunio-kun Nekketsu Collection 2 (Japan) | B9BJ | None | ok |  |
| Kunio-kun Nekketsu Collection 3 (Japan) | B9CJ | None | ok |  |
| Kurohige no Golf Shiyouyo (Japan) | AGOJ | Sram32K | ok |  |
| Kurohige no Kurutto Jintori (Japan) | AKUJ | Sram32K | ok |  |
| Kurukuru Kururin (Europe) | AKRP | Sram32K | ok |  |
| Kururin Paradise (Japan) | A9QJ | Sram32K | ok |  |
| Lady Sia (USA) (En,Fr,De,Es,It,Nl) | ALDE | Eeprom | ok |  |
| Land Before Time, The (USA) (En,Es) | ALAE | None | ok |  |
| Land Before Time, The - Into the Mysterious Beyond (USA) (En,Fr,Es) | BLOE | Eeprom | ok |  |
| Lara Croft Tomb Raider - Legend (USA) (En,Fr,De,Es,It) | BL8E | Eeprom | ok |  |
| Lara Croft Tomb Raider - The Prophecy (USA) (En,Fr,De,Es,It) | AL9E | None | ok |  |
| Legend of Dynamic - Goushouden - Houkai no Rondo (Japan) | AVDJ | Sram32K | ok |  |
| Legend of Spyro, The - A New Beginning (USA) | B3YE | Eeprom | ok |  |
| Legend of Spyro, The - The Eternal Night (USA) (En,Fr) | BU7E | Eeprom | ok |  |
| Legend of Zelda, The - A Link to the Past & Four Swords (USA) | AZLE | Eeprom | ok |  |
| Legend of Zelda, The - The Minish Cap (USA) | BZME | Eeprom | ok |  |
| Legends of Wrestling II (USA, Europe) | A2LE | Eeprom | ok |  |
| Legendz - Sign of Nekuromu (Japan) | BLVJ | Flash64K | ok | cartridge RTC (since 2026-10-03) |
| Legendz - Yomigaeru Shiren no Shima (Japan) | BLJJ | Flash64K | ok | cartridge RTC (since 2026-10-03) |
| LEGO Bionicle (USA) (En,Fr) | ALBE | Sram32K | ok |  |
| LEGO Island 2 - The Brickster's Revenge (USA) (En,Fr) | AL2E | Sram32K | ok |  |
| LEGO Racers 2 (USA) (En,Fr) | ALRE | Eeprom | ok |  |
| LEGO Star Wars - The Video Game (USA, Europe) (En,Fr,De,Es,It,Nl,Da) | BLWE | Eeprom | ok |  |
| LEGO Star Wars II - The Original Trilogy (USA) | BL7E | Eeprom | ok |  |
| Lemony Snicket's A Series of Unfortunate Events (USA, Europe) | BLYE | Eeprom | ok |  |
| Let's Ride! - Dreamer (USA) | BL9E | Eeprom | ok |  |
| Let's Ride! - Friends Forever (USA) | BEFE | Eeprom | ok |  |
| Let's Ride! - Sunshine Stables (USA) | B34E | Eeprom | ok |  |
| Licca-chan no Oshare Nikki (Japan) | BRNJ | Sram32K | ok |  |
| Lilliput Oukoku - Lillimoni to Issho Puni! (Japan) | BRPJ | Eeprom | ok |  |
| Lilo & Stitch (USA) | ALTE | None | ok |  |
| Lilo & Stitch 2 - Haemsterviel Havoc (USA) | BLSE | Eeprom | ok |  |
| Lion King 1 1-2, The (USA) | BLKE | Eeprom | ok |  |
| Little Buster Q (Japan) | ALQJ | Sram32K | ok |  |
| Little Einsteins (USA) | BEIE | None | ok |  |
| Little League Baseball 2002 (USA) (En,Es) | ALCE | None | ok |  |
| Little Mermaid, The - Magic in Two Kingdoms (USA, Europe) (En,Fr,De,Es,It) | BN9E | Eeprom | ok |  |
| Little Patissier - Cake no Oshiro (Japan) | BLIJ | Eeprom | ok |  |
| Lizzie McGuire - On the Go! (USA) | BLME | Eeprom | ok | music through the HLE BIOS sound driver (2026-10-01); pinned in tests/compat.toml |
| Lizzie McGuire 3 - Homecoming Havoc (USA) | BL3E | Eeprom | ok |  |
| Lode Runner (Japan) | A39J | Eeprom | ok |  |
| Looney Tunes - Back in Action (USA, Europe) (En,Fr,De,Es,It) | BLTE | Eeprom | ok |  |
| Looney Tunes Double Pack (USA) | BLNE | Eeprom | ok |  |
| Lord of the Rings, The - The Fellowship of the Ring (USA) (Rev 1) | ALOE | Eeprom | ok |  |
| Lord of the Rings, The - The Return of the King (USA, Europe) (En,Fr,De,Es,It) | BLRE | Eeprom | ok |  |
| Lord of the Rings, The - The Third Age (USA, Europe) (En,Fr,De,Es,It) | B3AE | Eeprom | ok |  |
| Lord of the Rings, The - The Two Towers (USA, Europe) (En,Fr,De,Es,It,Nl) | ALPE | Eeprom | ok |  |
| Lost Vikings, The (USA) | ALVE | Eeprom | ok |  |
| Love Hina Advance - Shukufuku no Kane wa Naru Kana (Japan) | ALHJ | Eeprom | ok |  |
| Lucky Luke - Wanted! (Europe) (En,Fr,De,Es,It,Nl) | ALLP | None | ok |  |
| Lufia - The Ruins of Lore (USA) | AGDE | Sram32K | ok |  |
| Lunar Legend (USA) | ALNE | Eeprom | ok |  |
| M&M's - Blast! (USA) | AMLE | Eeprom | ok |  |
| M&M's - Break' Em (USA) (Rev 1) | BEME | None | ok |  |
| Madagascar (USA) | BGZE | Eeprom | ok |  |
| Madagascar - Operation Penguin (USA) | BM7E | Eeprom | ok |  |
| Madden NFL 06 (USA) | B6ME | Eeprom | ok |  |
| Madden NFL 07 (USA) | B7ME | Eeprom | ok |  |
| Madden NFL 2002 (USA) | A2ME | Eeprom | ok |  |
| Madden NFL 2003 (USA) | ANJE | Eeprom | ok |  |
| Madden NFL 2004 (USA) | BMDE | Eeprom | ok |  |
| Madden NFL 2005 (USA) | BMFE | Eeprom | ok |  |
| MAER Heaven - Knockin' on Heaven's Door (Japan) | BM9J | Sram32K | ok |  |
| Magi Nation (Japan) | A2IJ | Eeprom | ok |  |
| Magical Houshin (Japan) | AJOJ | Flash64K | ok |  |
| Magical Quest 2 Starring Mickey & Minnie (USA) (En,Fr,De) | AQME | Eeprom | ok |  |
| Magical Quest 3 Starring Mickey & Donald (USA) (En,Fr,De) | BMQE | Eeprom | ok |  |
| Magical Quest Starring Mickey & Minnie (USA) | A3ME | Eeprom | ok |  |
| Magical Vacation (Japan) | AMVJ | Flash64K | ok |  |
| Mahjong Keiji (Japan) | AMPJ | Sram32K | ok |  |
| Mahou Sensei Negima! - Private Lesson - Damedesuu Toshokan-jima (Japan) | BNGJ | Sram32K | ok |  |
| Mahou Sensei Negima! - Private Lesson 2 - Ojamashimasuu Parasite de Chuu (Japan) | BNMJ | Sram32K | ok |  |
| Mail de Cute (Japan) | AMCJ | Flash64K | ok |  |
| Majesco's Sports Pack (USA) | B3NE | None | ok |  |
| Majokko Cream-chan no Gokko Series 1 - Wannyan Idol Gakuen (Japan) | BWAJ | Eeprom | ok |  |
| Majokko Cream-chan no Gokko Series 2 - Kisekae Angel (Japan) | BE2J | Eeprom | ok |  |
| Manga-ka Debut Monogatari - Akogare! Manga Ka Ikusei Game! (Japan) (Rev 1) | ACOJ | Eeprom | ok |  |
| Manic Miner (Europe) (En,Fr,De,Es,It) | A4MP | None | ok |  |
| March of the Penguins (USA) | BQLE | None | ok |  |
| Marie, Elie & Anis no Atelier - Soyokaze kara no Dengon (Japan) | ANSJ | Eeprom | ok |  |
| Mario & Luigi - Superstar Saga (USA) | A88E | Eeprom | ok |  |
| Mario Golf - Advance Tour (USA) | BMGE | Sram32K | ok |  |
| Mario Kart - Super Circuit (USA) | AMKE | Flash64K | ok |  |
| Mario Party Advance (USA) | B8ME | Eeprom | ok |  |
| Mario Pinball Land (USA, Australia) | BMVE | Eeprom | ok |  |
| Mario Tennis - Power Tour (USA, Australia) (En,Fr,De,Es,It) | BTME | Sram32K | ok |  |
| Mario vs. Donkey Kong (USA, Australia) | BM5E | Flash64K | ok |  |
| Marvel - Ultimate Alliance (USA) | B4ME | Eeprom | ok |  |
| Mary-Kate and Ashley - Girls Night Out (USA, Europe) | AKSE | Eeprom | ok |  |
| Mary-Kate and Ashley - Sweet 16 - Licensed to Drive (USA, Europe) | AAYE | None | ok |  |
| Masters of the Universe - He-Man - Power of Grayskull (USA) | AGUE | None | ok |  |
| Mat Hoffman's Pro BMX (USA, Europe) | AHOE | Eeprom | ok |  |
| Mat Hoffman's Pro BMX 2 (USA, Europe) | AH2E | Eeprom | ok |  |
| Matantei Loki Ragnarok - Gensou no Labyrinth (Japan) (Rev 1) | BMRJ | Eeprom | ok |  |
| Matchbox Cross Town Heroes (USA) | ARQE | None | ok |  |
| Math Patrol - The Kleptoid Threat (USA) | BIYE | Eeprom | ok |  |
| Max Payne (USA) | BMEE | Eeprom | ok |  |
| Maya the Bee - Sweet Gold (Europe) (En,Fr,De,Es,It) | BEEP | Eeprom | ok |  |
| Maya the Bee - The Great Adventure (Europe) (En,Fr,De,Es,It) | ABVP | None | ok |  |
| Mazes of Fate (USA) (En,Fr,De,Es,It) | BFQE | Eeprom | ok |  |
| Mech Platoon (USA) | AKGE | Eeprom | ok |  |
| Medabots - Metabee (USA) | A8BE | Eeprom | ok |  |
| Medabots - Rokusho (USA) | A9BE | Eeprom | ok |  |
| Medabots AX - Metabee Ver. (USA) | AK8E | Eeprom | ok |  |
| Medabots AX - Rokusho Ver. (USA) | AK9E | Eeprom | ok |  |
| Medal of Honor - Infiltrator (USA, Europe) (En,Fr,De) | BMHE | Eeprom | ok |  |
| Medal of Honor - Underground (USA) | AUGE | None | ok |  |
| Medarot Navi - Kabuto (Japan) | ANAJ | Flash64K | ok |  |
| Medarot Navi - Kuwagata (Japan) | AVIJ | Flash64K | ok |  |
| Meet the Robinsons (USA) | BRHE | Eeprom | ok |  |
| Megaman & Bass (USA) | A6ME | Eeprom | ok |  |
| Megaman - Battle Chip Challenge (USA) | A89E | Sram32K | ok |  |
| Megaman - Battle Network (USA) | AREE | Sram32K | ok |  |
| Megaman - Battle Network 2 (USA) | AE2E | Sram32K | ok |  |
| Megaman - Battle Network 3 - Blue Version (USA) | A3XE | Sram32K | ok |  |
| Megaman - Battle Network 3 - White Version (USA) | A6BE | Sram32K | ok |  |
| Megaman - Battle Network 4 - Blue Moon (USA) | B4BE | Sram32K | ok |  |
| Megaman - Battle Network 4 - Red Sun (USA) | B4WE | Sram32K | ok |  |
| Megaman - Battle Network 5 - Team Colonel (USA) | BRKE | Sram32K | ok |  |
| Megaman - Battle Network 5 - Team Protoman (USA) | BRBE | Sram32K | ok |  |
| Megaman - Battle Network 6 - Cybeast Falzar (USA) | BR6E | Sram32K | ok |  |
| Megaman - Battle Network 6 - Cybeast Gregar (USA) | BR5E | Sram32K | ok |  |
| Megaman Zero (USA, Europe) | AZCE | Sram32K | ok |  |
| Megaman Zero 2 (USA) | A62E | Sram32K | ok |  |
| Megaman Zero 3 (USA) | BZ3E | Sram32K | ok |  |
| Megaman Zero 4 (USA) | B4ZE | Eeprom | ok |  |
| Meitantei Conan - Akatsuki no Monument (Japan) | BQAJ | Eeprom | ok |  |
| Meitantei Conan - Nerawareta Tantei (Japan) | AC4J | Eeprom | ok |  |
| Men in Black - The Series (USA) | AMIE | None | ok |  |
| Mermaid Melody - Pichi Pichi Pitch (Japan) | BMAJ | Sram32K | ok |  |
| Mermaid Melody - Pichi Pichi Pitch - Pichi Pichi Party (Japan) | BM8J | Sram32K | ok |  |
| Mermaid Melody - Pichi Pichi Pitch - Pichi Pichitto Live Start! (Japan) | B3MJ | Sram32K | ok |  |
| Metal Max 2 Kai (Japan) (Rev 1) | A9TJ | Eeprom | ok |  |
| Metal Slug Advance (USA) | BSME | Eeprom | ok |  |
| Metalgun Slinger (Japan) | AAPJ | Eeprom | ok |  |
| Metroid - Zero Mission (USA) | BMXE | Sram32K | ok |  |
| Metroid Fusion (USA) | AMTE | Sram32K | ok |  |
| Mezase! Koushien (Japan) | BMKJ | Flash64K | ok |  |
| Micro Machines (Europe) (En,Fr,De,Es,It) | AXZP | None | ok |  |
| Midnight Club - Street Racing (USA) | AMQE | None | ok |  |
| Midway's Greatest Arcade Hits (USA, Europe) | AM3E | None | ok |  |
| Mighty Beanz - Pocket Puzzles (USA) | BMBE | Eeprom | ok |  |
| Mijn Dierenpraktijk (Netherlands) (En,Nl) | BQVX | Eeprom | ok |  |
| Mike Tyson Boxing (USA) (En,Fr,De,Es,It) | AM6E | Eeprom | ok |  |
| Minami no Umi no Odyssey (Japan) | AODJ | Eeprom | ok |  |
| Mini Moni. - Mika no Happy Morning Chatty (Japan) | AHCJ | Sram32K | ok |  |
| Mini Moni. - Onegai Ohoshi-sama! (Japan) | AOHJ | Eeprom | ok |  |
| Minna no Ouji-sama (Japan) | BMOJ | Sram32K | ok |  |
| Minna no Shiiku Series - Boku no Kabuto, Kuwagata (Japan) | BKKJ | Eeprom | ok |  |
| Minna no Shiiku Series 1 - Boku no Kabutomushi (Japan) | AB7J | Sram32K | ok |  |
| Minna no Shiiku Series 2 - Boku no Kuwagata (Japan) | AW7J | Sram32K | ok |  |
| Minna no Soft Series - Happy Trump 20 (Japan) | BTLJ | Eeprom | ok |  |
| Minna no Soft Series - Hyokkori Hyoutan-jima - Don Gabacho Daikatsuyaku no Maki (Japan) | BHYJ | Eeprom | ok |  |
| Minna no Soft Series - Minna no Mahjong (Japan) | BMJJ | Eeprom | ok |  |
| Minna no Soft Series - Minna no Shougi (Japan) (Rev 1) | BSGJ | Eeprom | waiting | menu waiting for input (challenge setup) |
| Minna no Soft Series - Shanghai (Japan) (Rev 1) | BSHJ | Eeprom | ok |  |
| Minna no Soft Series - Tetris Advance (Japan) (Rev 1) | BTTJ | Eeprom | ok |  |
| Minority Report - Everybody Runs (USA, Europe) | ARME | Eeprom | ok |  |
| Mirakuru! Panzou - 7-tsu no Hoshi no Uchuu Kaizoku (Japan) | B3IJ | Eeprom | ok |  |
| Mission Impossible - Operation Surma (USA) (En,Fr,Es) | AIHE | Eeprom | ok |  |
| MLB SlugFest 2004 (USA) | A5ME | None | ok |  |
| Mobile Pro Yakyuu - Kantoku no Saihai (Japan) | AMBJ | Flash64K | ok |  |
| Mobile Suit Gundam Seed - Battle Assault (USA) | BGNE | None | ok |  |
| Moero!! Jaleco Collection (Japan) | BJCJ | Eeprom | ok |  |
| Momotarou Dentetsu G - Gold Deck o Tsukure! (Japan) | BM2J | Eeprom | ok |  |
| Momotarou Matsuri (Japan) (Rev 1) | AMMJ | Sram32K | ok |  |
| Monopoly (USA) | BUME | None | ok |  |
| Monster Force (USA) | AM8E | Eeprom | ok |  |
| Monster Gate (Japan) (Rev 1) | ANFJ | Sram32K | ok |  |
| Monster Gate - Ooinaru Dungeon - Fuuin no Orb (Japan) | A6GJ | Sram32K | ok |  |
| Monster Guardians (Japan) | AMNJ | Sram32K | ok |  |
| Monster House (USA) (En,Fr) | BQ7E | Eeprom | ok |  |
| Monster Jam - Maximum Destruction (USA) | AJAE | Sram32K | ok |  |
| Monster Maker 4 - Flash Card (Japan) | AA4J | Sram32K | ok |  |
| Monster Maker 4 - Killer Dice (Japan) | AA5J | Sram32K | ok |  |
| Monster Rancher Advance (USA) | AMFE | Sram32K | ok |  |
| Monster Rancher Advance 2 (USA) | A2QE | Sram32K | ok |  |
| Monster Summoner (Japan) | A3NJ | Sram32K | ok |  |
| Monster Truck Madness (USA, Europe) | BMTE | Eeprom | ok |  |
| Monster Trucks (USA, Europe) | BMCE | None | ok |  |
| Monster Trucks Mayhem (USA) | BYME | None | ok |  |
| Monster! Bass Fishing (USA) | A4BE | None | ok |  |
| Monsters, Inc. (USA, Europe) | AMXE | None | ok |  |
| Moorhen 3 - The Chicken Chase! (Europe) (En,Fr,De,Es,It) | AU3P | Eeprom | ok |  |
| Morita Shougi Advance (Japan) | AMSJ | Sram32K | ok |  |
| Mortal Kombat - Deadly Alliance (USA) (En,Fr,De,Es,It) | AXDE | Eeprom | ok |  |
| Mortal Kombat - Tournament Edition (USA) (En,Fr,De,Es,It) | AW4E | Eeprom | ok |  |
| Mortal Kombat Advance (USA) | AM5E | None | ok |  |
| Mother 1+2 (Japan) | A2UJ | Sram32K | ok |  |
| Mother 3 (Japan) | A3UJ | Flash64K | ok |  |
| Moto Racer Advance (USA) (En,Fr,De,Es,It) | A9ME | Sram32K | ok |  |
| Motocross Maniacs Advance (USA) (En,Es) | AMRE | Eeprom | ok |  |
| MotoGP (USA) (En,Fr,De,Es,It) | AM4E | None | ok |  |
| Mr Nutz (Europe) (En,Fr,De,Es,It) | AZRP | None | ok |  |
| Mr. Driller 2 (USA) | BR2E | Sram32K | ok |  |
| Mr. Driller A - Fushigi na Pacteria (Japan) | AD5J | Sram32K | ok |  |
| Ms. Pac-Man - Maze Madness (USA) | BPCE | Eeprom | ok |  |
| Mucha Lucha! - Mascaritas of the Lost Code (USA) (En,Fr,Es) | BMLE | Eeprom | ok |  |
| Mugenborg (Japan) | AG6J | Sram32K | ok |  |
| Mummy, The (USA) (En,Fr,De,Es,It) | AUME | None | ok |  |
| Muppet Pinball Mayhem (USA) | AMWE | Eeprom | ok |  |
| Muppets, The - On with the Show! (USA, Europe) (En,Fr,De,Es,It,Nl) | AZME | None | ok |  |
| Music Recorder MP3 (Japan) | ZMPJ | None | needs-hardware | needs its MP3 cartridge hardware (shows the battery error on purpose) |
| Mutsu - Water Looper Mutsu (Japan) | AMUJ | Sram32K | ok |  |
| MX 2002 Featuring Ricky Carmichael (USA, Europe) | A2XE | None | ok |  |
| My Animal Centre in Africa (Europe) (En,Fr,De,Es,It) | BFRP | Eeprom | ok |  |
| My Little Pony - Crystal Princess - The Runaway Rainbow (USA) | BL6E | None | ok |  |
| Nakayoshi Mahjong - KabuReach (Japan) | AKPJ | Sram32K | ok |  |
| Nakayoshi Pet Advance Series 1 - Kawaii Hamster (Japan) | AH7J | Eeprom | ok |  |
| Nakayoshi Pet Advance Series 2 - Kawaii Koinu (Japan) | AI7J | Eeprom | ok |  |
| Nakayoshi Pet Advance Series 4 - Kawaii Koinu Mini - Wanko to Asobou!! Kogata-ken (Japan) | BKIJ | Eeprom | ok |  |
| Nakayoshi Youchien - Sukoyaka Enji Ikusei Game (Japan) (Rev 1) | AHVJ | Eeprom | ok |  |
| Namco Museum (USA) | ANME | None | ok |  |
| Namco Museum - 50th Anniversary (USA) | B5NE | None | ok |  |
| Nancy Drew - Message in a Haunted Mansion (USA) | ANDE | None | waiting | in-game, waiting for input (Chinese Room) |
| Napoleon (Japan) | ANPJ | Sram32K | ok |  |
| Narikiri Jockey Game - Yuushun Rhapsody (Japan) | AYRJ | Sram32K | ok |  |
| Naruto - Konoha Senki (Japan) (Rev 1) | AUEJ | Sram32K | ok |  |
| Naruto - Ninja Council (USA) | A7AE | Eeprom | ok |  |
| Naruto - Ninja Council 2 (USA) | BN2E | Sram32K | ok |  |
| Naruto RPG - Uketsugareshi Hi no Ishi (Japan) (Rev 1) | BNRJ | Sram32K | ok |  |
| NASCAR Heat 2002 (USA) | ANHE | Eeprom | ok |  |
| Natural 2 - Duo (Japan) | AN2J | Flash64K | ok |  |
| NBA Jam 2002 (USA, Europe) | ABNE | None | ok |  |
| Need for Speed - Carbon - Own the City (USA, Europe) (En,Fr,De,Es,It) | BN7E | Eeprom | ok |  |
| Need for Speed - Most Wanted (USA, Europe) (En,Fr,De,It) | BNWE | Eeprom | ok |  |
| Need for Speed - Porsche Unleashed (USA) | AZFE | Eeprom | ok |  |
| Need for Speed - Underground (USA, Europe) (En,Fr,De,It) | BNSE | Eeprom | ok |  |
| Need for Speed - Underground 2 (USA, Europe) (En,Fr,De,It) | BNFE | Eeprom | ok |  |
| Neoromance Game - Harukanaru Toki no Naka de (Japan) (Rev 1) | ARNJ | Eeprom | ok |  |
| Next Generation Tennis (Europe) (En,Fr,De,Es,It,Pt) | ATXP | Eeprom | ok |  |
| NFL Blitz 2002 (USA) | ABZE | None | ok |  |
| NFL Blitz 2003 (USA) | ANKE | None | ok |  |
| NHL 2002 (USA) | ANLE | Eeprom | ok |  |
| NHL Hitz 2003 (USA) | AN4E | Eeprom | waiting | menu waiting for input (choose sides) |
| Nicktoons - Attack of the Toybots (USA) | BUJE | None | ok |  |
| Nicktoons - Battle for Volcano Island (USA) | BNVE | Eeprom | ok |  |
| Nicktoons - Freeze Frame Frenzy (USA) | BCCE | Eeprom | ok |  |
| Nicktoons Racing (USA) | ANQE | Eeprom | ok |  |
| Nicktoons Unite! (USA) | BNUE | None | ok |  |
| Nihon Pro Mahjong Renmei Kounin - Tetsuman Advance - Menkyo Kaiden Series (Japan) | ANTJ | Eeprom | ok |  |
| Ninja Five-0 (USA) | ANXE | Eeprom | ok |  |
| No No No Puzzle Chailien (Japan) | BKRJ | Eeprom | ok |  |
| No Rules - Get Phat (USA, Europe) (En,Fr,De,Es,It,Nl) | AGPE | None | ok |  |
| Nobunaga Ibun (Japan) | ANOJ | Sram32K | ok |  |
| Nobunaga no Yabou (Japan) | ANBJ | Flash64K | ok |  |
| Noddy - A Day in Toyland (USA) (En,Fr,Es) | BNKE | None | ok |  |
| Nyan Nyan Nyanko no NyanCollection (Japan) | BNYJ | Eeprom | ok |  |
| Ochaken Kururin - Honwaka Puzzle de Hotto Shiyo (Japan) | BIKJ | Eeprom | ok |  |
| Ochaken no Bouken-jima - Honwaka Yume no Island (Japan) | BO2J | Eeprom | ok |  |
| Ochaken no Heya (Japan) | BDRJ | Eeprom | ok |  |
| Ochaken no Yume Bouken (Japan) | BCUJ | Eeprom | ok |  |
| Oddworld - Munch's Oddysee (USA, Europe) | BODE | None | ok |  |
| Ohanaya-san Monogatari GBA - Iyashikei Ohanaya-san Ikusei Game (Japan) | A87J | Eeprom | ok |  |
| Ojarumaru - Gekkouchou Sanpo de Ojaru (Japan) | BOJJ | Eeprom | ok |  |
| Okumanchouja Game - Nottori Daisakusen! (Japan) | AOKJ | Sram32K | ok |  |
| One Piece (USA) | BONE | Eeprom | ok |  |
| One Piece - Dragon Dream (Japan) | BIPJ | Eeprom | ok |  |
| One Piece - Going Baseball - Kaizoku Yakyuu (Japan) | B08J | Eeprom | ok |  |
| Onimusha Tactics (USA) | A6OE | Eeprom | ok |  |
| Onmyou Taisenki - Zeroshiki (Japan) | BITJ | Eeprom | ok |  |
| Open Season (USA) (En,Fr,Es) | BOAE | Eeprom | ok |  |
| Operation Armored Liberty (USA) | BAAE | None | ok |  |
| Oriental Blue - Ao no Tengai (Japan) | AORJ | Sram32K | ok |  |
| Oshaberi Inko Club (Japan) | AICJ | Flash64K | ok |  |
| Oshare Princess (Japan) | AOPJ | Eeprom | ok |  |
| Oshare Princess 2 + Doubutsu Kyaranabi Uranai (Japan) | AO2J | Eeprom | ok |  |
| Oshare Princess 3 (Japan) | BO3J | Eeprom | ok |  |
| Oshare Princess 5 (Japan) | BO5J | Eeprom | ok |  |
| Ottifanten Pinball (Germany) | BOFD | None | ok |  |
| Over the Hedge (USA) | BH5E | Eeprom | ok |  |
| Over the Hedge - Hammy Goes Nuts! (USA) | BH7E | Eeprom | ok |  |
| Ozzy & Drix (USA) | BOZE | None | ok |  |
| Pac-Man Collection (USA) | APCE | None | ok |  |
| Pac-Man Pinball Advance (USA) | BP8E | None | ok |  |
| Pac-Man World (USA) | BPAE | Eeprom | ok |  |
| Pac-Man World 2 (USA) | B2CE | None | ok |  |
| Paws & Claws - Best Friends - Dogs & Cats (USA) | BHBE | Eeprom | ok |  |
| Paws & Claws - Pet Resort (USA) | BURE | Eeprom | ok |  |
| Paws & Claws - Pet Vet (USA) | BQVE | Eeprom | ok |  |
| Payback (Europe) (En,Fr,De,Es,It) | BPKP | Eeprom | ok |  |
| Peter Pan - Return to Neverland (USA) (Rev 1) | APPE | None | ok |  |
| Peter Pan - The Motion Picture Event (USA) | BPTE | Eeprom | ok |  |
| Petz - Hamsterz Life 2 (USA) | AJHE | Sram32K | ok |  |
| Petz Vet (USA) | BNBE | Eeprom | ok |  |
| Phalanx (USA) | APXE | Eeprom | ok |  |
| Phantasy Star Collection (USA) | AYCE | Eeprom | ok |  |
| Phil of the Future (USA) | BFXE | Eeprom | ok |  |
| Pia Carrot e Youkoso!! 3.3 (Japan) | BP3J | Eeprom | ok |  |
| Piglet's Big Game (USA) | A9NE | None | ok |  |
| Pikapika Nurse Monogatari - Nurse Ikusei Game (Japan) | BPNJ | Eeprom | ok |  |
| Pinball Advance (Europe) (En,Fr,De,Es,It) | APZP | Eeprom | ok |  |
| Pinball Challenge Deluxe (Europe) | APLP | None | ok |  |
| Pinball of the Dead, The (USA) | APDE | Eeprom | ok |  |
| Pinball Tycoon (USA) | A2TE | None | ok |  |
| Pink Panther - Pinkadelic Pursuit (USA) | APEE | None | ok |  |
| Pinky and the Brain - The Master Plan (Europe) (En,Fr,De,Es,It) | APIP | None | ok |  |
| Pinky Monkey Town (Japan) | APNJ | Eeprom | ok |  |
| Pinobee & Phoebee (Japan) | AP6J | Eeprom | ok |  |
| Pinobee - Wings of Adventure (USA, Europe) | APBE | Sram32K | ok |  |
| Pirates of the Caribbean - Dead Man's Chest (USA, Europe) (En,Fr,De,Es,It) | B8QE | Eeprom | ok |  |
| Pirates of the Caribbean - The Curse of the Black Pearl (USA) (En,Fr,De,Es,It) | A8QE | None | ok |  |
| Pitfall - The Lost Expedition (USA) | BPHE | Eeprom | ok |  |
| Pitfall - The Mayan Adventure (USA, Europe) | APFE | None | ok |  |
| Pixeline i Pixieland (Denmark) | BQ9Q | Eeprom | ok |  |
| Planet Monsters (USA) (En,Fr,De,Es,It,Nl) | APME | None | ok |  |
| Planet of the Apes (USA) (En,Fr,De,Es,It,Nl) | AYNE | None | ok |  |
| Play Novel - Silent Hill (Japan) | ASHJ | Sram32K | ok |  |
| Pocket Dogs (USA) | BTDE | Eeprom | ok |  |
| Pocket Music (Europe) (En,Fr,De,Es,It) | AP9P | Sram32K | ok |  |
| Pocket Professor - Kwik Notes - Volume One (USA) | BPJE | Eeprom | ok | speech clip through the HLE BIOS sound driver (2026-10-01; fixed-rate voice); pinned in tests/compat.toml |
| Pocky & Rocky with Becky (USA) | APKE | None | ok |  |
| Pokemon - Emerald Version (USA, Europe) | BPEE | Flash128K | ok | cartridge RTC (since 2026-10-03; Emerald pinned in compat.toml) |
| Pokemon - FireRed Version (USA, Europe) (Rev 1) | BPRE | Flash128K | ok |  |
| Pokemon - LeafGreen Version (USA, Europe) (Rev 1) | BPGE | Flash128K | ok |  |
| Pokemon - Ruby Version (USA, Europe) (Rev 2) | AXVE | Flash128K | ok | cartridge RTC (since 2026-10-03) |
| Pokemon - Sapphire Version (USA, Europe) (Rev 2) | AXPE | Flash128K | ok | cartridge RTC (since 2026-10-03) |
| Pokemon Mystery Dungeon - Red Rescue Team (USA, Australia) | B24E | Flash128K | ok |  |
| Pokemon Pinball - Ruby & Sapphire (USA) | BPPE | Sram32K | ok |  |
| Polar Express, The (USA, Europe) (En,Fr,De,Es,It) | BPXE | Eeprom | ok |  |
| Polarium Advance (USA) | BIIE | Flash64K | ok |  |
| Polly Pocket! - Super Splash Island (USA) (DSI) | B3FE | None | ok |  |
| Popeye - Rush for Spinach (USA, Europe) (En,Fr,De,Es,It) | APOE | None | ok |  |
| Postman Pat and the Greendale Rocket (Europe) (En,No,Da) | BROP | None | ok |  |
| Power Poke Dash (Japan) | BBQJ | Sram32K | ok |  |
| Power Pro Kun Pocket 1, 2 (Japan) | B8PJ | Sram32K | ok |  |
| Power Pro Kun Pocket 3 (Japan) (Rev 1) | AP3J | Sram32K | ok |  |
| Power Pro Kun Pocket 4 (Japan) (Rev 1) | AP4J | Sram32K | ok |  |
| Power Pro Kun Pocket 5 (Japan) (Rev 1) | A5PJ | Sram32K | ok |  |
| Power Pro Kun Pocket 6 (Japan) | BP6J | Sram32K | ok |  |
| Power Pro Kun Pocket 7 (Japan) | BP7J | Sram32K | ok |  |
| Power Rangers - Dino Thunder (USA, Europe) | BPOE | None | ok |  |
| Power Rangers - Ninja Storm (USA) | BPWE | None | ok |  |
| Power Rangers - Time Force (USA, Europe) | APRE | None | ok |  |
| Power Rangers - Wild Force (USA, Europe) | APWE | None | ok |  |
| Power Rangers S.P.D. (USA, Europe) | BRDE | None | ok |  |
| Powerpuff Girls, The - Him and Seek (USA) | AP5E | Eeprom | ok |  |
| Powerpuff Girls, The - Mojo Jojo A-Go-Go (USA) (En,Fr,De,Es,It,Nl) | APTE | Eeprom | ok |  |
| Prehistorik Man (USA, Europe) (En,Fr,De,Es,It,Nl) | APHE | None | ok |  |
| Premier Action Soccer (Europe) (En,Fr,De,Es,It) | BAQP | Eeprom | ok |  |
| Premier Manager 2003-04 (Europe) (En,Fr,De,It) (Rev 1) | BPMP | Flash64K | ok |  |
| Premier Manager 2004-2005 (Europe) (En,Fr,De,It) | BP4P | Flash64K | ok |  |
| Premier Manager 2005-2006 (Europe) (En,Fr,De,It) | BP5P | Flash64K | ok |  |
| Prince of Persia - The Sands of Time (USA) (En,Fr,Es) (Rev 1) | BPYE | Eeprom | ok |  |
| Princess Natasha - Student, Secret Agent, Princess (USA) | BNPE | None | ok |  |
| Pro Mahjong Tsuwamono GBA (Japan) | B2OJ | Eeprom | ok |  |
| Pro Yakyuu Team o Tsukurou! Advance (Japan) | ALMJ | Sram32K | ok |  |
| Proud Family, The (USA) | BD7E | Eeprom | ok |  |
| PukuPuku Tennen Kairanban (Japan) | APUJ | Eeprom | ok |  |
| PukuPuku Tennen Kairanban - Koi no Cupid Daisakusen (Japan) | BPQJ | Eeprom | ok |  |
| PukuPuku Tennen Kairanban - Youkoso! Illusion Land he (Japan) | B3PJ | Eeprom | ok |  |
| Punch King - Arcade Boxing (USA) | APGE | None | ok |  |
| Puppy Luv - Spa and Resort (USA) | BYXE | Eeprom | ok |  |
| Puyo Pop (USA) (En,Ja) | APYE | Eeprom | ok |  |
| Puyo Pop Fever (Europe) (En,Fr,De,Es,It) | BPFP | Eeprom | ok |  |
| Puzzle & Tantei Collection (Japan) | AEHJ | Eeprom | ok |  |
| Pyuu to Fuku! Jaguar - Byoo to Deru! Megane-kun (Japan) | BPBJ | Sram32K | ok |  |
| Quad Desert Fury (USA, Europe) | BQDE | None | ok |  |
| R-Type III - The Third Lightning (USA) | BR3E | None | ok |  |
| Racing Fever (Europe) (En,De,Es,It) | BRWP | None | ok |  |
| Racing Gears Advance (USA) | BRAE | Eeprom | ok |  |
| Rampage - Puzzle Attack (USA, Europe) | ARXE | None | ok | music through the HLE BIOS sound driver (2026-10-01; fixed-rate voices); pinned in tests/compat.toml |
| Rapala Pro Fishing (USA, Europe) | BRFE | Eeprom | ok |  |
| Ratatouille (USA) | BNLE | None | ok |  |
| Rave Master - Special Attack Force! (USA) | BRME | Sram32K | ok |  |
| Rayman - 10th Anniversary (USA) (En,Fr,De,Es,It) | BX5E | Eeprom | ok |  |
| Rayman - Hoodlum's Revenge (USA) (En,Fr,Es) | BRYE | Eeprom | ok |  |
| Rayman - Raving Rabbids (USA) (En,Fr,Es) | BQ3E | Eeprom | ok |  |
| Rayman 3 (USA) (En,Fr,Es) | AYZE | Eeprom | ok |  |
| Rayman Advance (USA) (En,Fr,De,Es,It) | ARYE | Eeprom | ok |  |
| Razor Freestyle Scooter (USA) | ARFE | None | ok |  |
| Ready 2 Rumble Boxing - Round 2 (USA) | AR2E | None | ok |  |
| Rebelstar - Tactical Command (USA) | BRLE | Eeprom | ok |  |
| Recca no Honoo - The Game (Japan) | ARHJ | Sram32K | ok |  |
| Reign of Fire (USA) (En,Fr,De,Es,It) | AR9E | None | ok |  |
| Relaxuma na Mainichi (Japan) | BR9J | Eeprom | ok |  |
| Rescue Heroes - Billy Blazes! (USA) | AQHE | None | ok |  |
| Revenge of Shinobi, The (USA) | A3RE | None | ok |  |
| Revenge of the Smurfs, The (Europe) (En,Fr,De,Es,It,Nl) | A7SP | None | ok |  |
| Rhythm Tengoku (Japan) (Rev 1) | BRIJ | Sram32K | ok |  |
| Ripping Friends, The - The World's Most Manly Men! (USA, Europe) | ARDE | None | ok |  |
| River City Ransom EX (USA) | BDTE | Eeprom | ok |  |
| Riviera - The Promised Land (USA) | BREE | Sram32K | ok |  |
| Road Rash - Jailbreak (USA) | A9RE | None | ok |  |
| Robopon 2 - Cross Version (USA) | ACVE | Flash64K | ok |  |
| Robopon 2 - Ring Version (USA) | ARPE | Flash64K | ok |  |
| Robot Wars - Advanced Destruction (USA) | ARUE | Eeprom | ok |  |
| Robot Wars - Extreme Destruction (Europe) (En,Fr,De,Es,It,Nl) | ARSP | Eeprom | ok |  |
| Robotech - The Macross Saga (USA, Europe) (En,Fr,De,Es,It,Nl) | ARBE | Eeprom | ok |  |
| Robots (USA) | BRTE | Eeprom | ok |  |
| Rock 'N Roll Racing (USA) | A4RE | Eeprom | ok |  |
| Rock'em Sock'em Robots (USA) | BR7E | None | ok |  |
| Rocket Power - Beach Bandits (USA, Europe) | AR4E | None | ok |  |
| Rocket Power - Dream Scheme (USA, Europe) | ARKE | None | ok |  |
| Rocket Power - Zero Gravity Zone (USA) | AZZE | None | ok |  |
| Rockman EXE 4.5 - Real Operation (Japan) | BR4J | Flash64K | ok | cartridge RTC (since 2026-10-03) |
| Rocky (USA) (En,Fr,De,Es,It) | AR8E | Eeprom | ok |  |
| RPG Tsukuru Advance (Japan) | A8TJ | Flash64K | ok |  |
| Rugrats - Castle Capers (USA, Europe) | ARGE | None | ok |  |
| Rugrats - Go Wild (USA, Europe) | A5WE | None | ok |  |
| Rugrats - I Gotta Go Party (USA, Europe) | AR5E | None | ok |  |
| Sabre Wulf (USA) | AWUE | Eeprom | ok |  |
| Sabrina the Teenage Witch - Potion Commotion (USA) (En,Fr,Es) | A3BE | None | ok |  |
| Saibara Rieko no Dendou Mahjong (Japan) | ASMJ | Sram32K | ok | music through the HLE BIOS sound driver and its SoundGetJumpList routines (2026-10-01); pinned in tests/compat.toml |
| Sakura Momoko no UkiUki Carnival (Japan) | ACLJ | Sram32K | ok |  |
| Salt Lake 2002 (USA) (En,Fr,De,Es,It,Nl) | AS5E | Eeprom | ok |  |
| Samsara Naga 1x2 (Japan) (Rev 2) | ASNJ | Eeprom | ok |  |
| Samurai Deeper Kyo (USA) | AOSE | Eeprom | ok |  |
| Samurai Evolution - Oukoku Geist (Japan) | AECJ | Sram32K | ok |  |
| Samurai Jack - The Amulet of Time (USA, Europe) | AJTE | Eeprom | ok |  |
| Sangokushi (Japan) | ASXJ | Flash64K | ok |  |
| Sangokushi - Eiketsuden (Japan) | B3EJ | Flash64K | ok |  |
| Sangokushi - Koumeiden (Japan) | B3QJ | Flash64K | ok |  |
| Sanrio Puroland - All Characters (Japan) | A85J | Eeprom | ok |  |
| Santa Claus Jr. Advance (Europe) | AXXP | Eeprom | ok |  |
| Santa Claus Saves the Earth (Europe) | AUZE | None | ok |  |
| Santa Clause 3, The - The Escape Clause (USA) | B33E | Eeprom | ok |  |
| Scan Hunter - Sennen Kaigyo o Oe! (Japan) | A57J | Sram32K | ok |  |
| Scooby-Doo (USA) | AP8E | Eeprom | ok |  |
| Scooby-Doo 2 - Monsters Unleashed (USA, Europe) | BMUE | None | ok |  |
| Scooby-Doo and the Cyber Chase (USA, Europe) | ASDE | None | ok |  |
| Scooby-Doo! - Mystery Mayhem (USA) (En,Fr) | BMME | Eeprom | ok |  |
| Scooby-Doo! - Unmasked (USA) (En,Fr) | B25E | None | ok |  |
| Scorpion King, The - Sword of Osiris (USA) | ASZE | None | ok |  |
| Scrabble (Europe) (En,Fr,De,Es) | AQBP | Sram32K | ok |  |
| Scrabble Blast! (USA) | BLAE | None | ok |  |
| Scurge - Hive (USA) (En,Fr,Es) | BHVE | Eeprom | ok |  |
| SD Gundam Force (USA) | BGEE | Eeprom | ok |  |
| SD Gundam G Generation Advance (Japan) | BGAJ | Eeprom | ok |  |
| Sea Trader - Rise of Taipan (USA) | ALJE | Eeprom | ok |  |
| Secret Agent Barbie - Royal Jewels Mission (USA) | AAHE | None | ok |  |
| SEGA Arcade Gallery (USA) | AYPE | None | ok |  |
| SEGA Rally Championship (USA) | AYLE | Sram32K | ok |  |
| SEGA Smash Pack (USA) | A3PE | None | ok |  |
| Sengoku Kakumei Gaiden (Japan) | A7GJ | Sram32K | ok |  |
| Sennen Kazoku (Japan) (Rev 1) | BKAJ | Flash128K | ok | cartridge RTC (since 2026-10-03) |
| Sentouin Yamada Hajime (Japan) | BSYJ | Sram32K | ok |  |
| Serious Sam Advance (USA) (En,Fr,De) | AENE | None | ok |  |
| Shaman King - Legacy of the Spirits - Soaring Hawk (USA) | BHLE | Sram32K | ok |  |
| Shaman King - Legacy of the Spirits - Sprinting Wolf (USA) | BWSE | Sram32K | ok |  |
| Shaman King - Master of Spirits (USA) | BSOE | Sram32K | ok |  |
| Shaman King - Master of Spirits 2 (USA) | B2ME | Sram32K | ok |  |
| Shaman King Card Game - Chou Senjiryakketsu 2 (Japan) | AKAJ | Sram32K | ok |  |
| Shaman King Card Game - Chou Senjiryakketsu 3 (Japan) | AL3J | Sram32K | ok |  |
| Shamu's Deep Sea Adventures (USA) | BBAE | Eeprom | ok |  |
| Shanghai Advance (Japan) | ASVJ | Eeprom | ok |  |
| Shark Tale (USA, Europe) | BSUE | Eeprom | ok |  |
| Shaun Palmer's Pro Snowboarder (USA, Europe) | ASCE | None | ok |  |
| Sheep (Europe) (En,Fr,De,Es,It) | AEPP | Eeprom | ok |  |
| Shifting Gears - Road Trip (USA) | A6RE | Sram32K | ok |  |
| Shikakui Atama o Maruku Suru. Advance - Kanji, Keisan (Japan) | B4KJ | Eeprom | ok |  |
| Shikakui Atama o Maruku Suru. Advance - Kokugo, Sansuu, Shakai, Rika (Japan) | B4RJ | Eeprom | ok |  |
| Shimura Ken no Baka Tonosama - Bakushou Tenka Touitsu Game (Japan) (Rev 1) | A64J | Eeprom | ok |  |
| Shin Bokura no Taiyou - Gyakushuu no Sabata (Japan) | U33J | Eeprom | ok | cartridge RTC + solar sensor (since 2026-10-03) |
| Shin Kisekae Monogatari (Japan) | AAJJ | Sram32K | ok |  |
| Shin Megami Tensei (Japan) | AAUJ | Sram32K | ok |  |
| Shin Megami Tensei - Devil Children - Honoo no Sho (Japan) | BDHJ | Sram32K | ok |  |
| Shin Megami Tensei - Devil Children - Koori no Sho (Japan) | BDYJ | Sram32K | ok |  |
| Shin Megami Tensei - Devil Children - Messiah Riser (Japan) | BDLJ | Sram32K | ok |  |
| Shin Megami Tensei - Devil Children - Puzzle de Call! (Japan) | A8ZJ | Eeprom | ok |  |
| Shin Megami Tensei II (Japan) | A5TJ | Sram32K | ok |  |
| Shin Nihon Pro Wrestling - Toukon Retsuden Advance (Japan) | ARAJ | Sram32K | ok |  |
| Shingata Medarot - Kabuto Version (Japan) | BKVJ | Eeprom | ok |  |
| Shingata Medarot - Kuwagata Version (Japan) | BKUJ | Eeprom | ok |  |
| Shining Force - Resurrection of the Dark Dragon (USA) | AF5E | Sram32K | ok |  |
| Shining Soul (USA) | AHUE | Flash64K | ok |  |
| Shining Soul II (USA) | AU2E | Flash64K | ok |  |
| Shiren Monsters - Netsal (Japan) | ANVJ | Sram32K | ok |  |
| Shrek - Hassle at the Castle (USA) (En,Fr,De,Es,It,Nl) | AH4E | Eeprom | ok |  |
| Shrek - Reekin' Havoc (USA) (En,Fr,De,Es,It,Nl) | AOIE | Eeprom | ok |  |
| Shrek - Smash n' Crash Racing (USA) | B4IE | Eeprom | ok |  |
| Shrek - Super Slam (USA) | B4UE | Eeprom | ok |  |
| Shrek - Swamp Kart Speedway (USA) (En,Fr,De,Es,It,Nl) (Rev 1) | AS4E | Eeprom | ok |  |
| Shrek 2 (USA, Europe) | BSEE | Eeprom | ok |  |
| Shrek 2 - Beg for Mercy (USA, Europe) | BSIE | Eeprom | ok |  |
| Shrek the Third (USA) | B3HE | Eeprom | ok |  |
| Sigma Star Saga (USA, Europe) | B3GE | Eeprom | ok |  |
| Silent Scope (USA) (En,Fr,De,Es,It) | AIPE | Eeprom | ok |  |
| Silk to Cotton (Japan) | A7IJ | Sram32K | ok |  |
| SimCity 2000 (USA) (Rev 1) | A5CE | Eeprom | ok |  |
| Simple 2960 Tomodachi Series Vol. 1 - The Table Game Collection - Mahjong, Shougi, Hanafuda, Reversi (Japan) | AZKJ | Eeprom | ok |  |
| Simple 2960 Tomodachi Series Vol. 2 - The Block Kuzushi (Japan) (Rev 1) | AZ9J | Eeprom | ok |  |
| Simple 2960 Tomodachi Series Vol. 3 - The Itsudemo Puzzle - Massugu Soroete Straws (Japan) | BS3J | Eeprom | ok |  |
| Simple 2960 Tomodachi Series Vol. 4 - The Trump - Minna de Asoberu 12 Shurui no Trump Game (Japan) (Rev 1) | BS4J | Eeprom | ok |  |
| Simpsons, The - Road Rage (USA, Europe) | A4AE | None | ok |  |
| Sims 2, The (USA, Europe) (En,Fr,De,Es,It,Nl) | B46E | Flash64K | ok |  |
| Sims 2, The - Pets (USA, Europe) | B4OE | Flash64K | ok |  |
| Sims, The - Bustin' Out (USA) (En,Fr,De,Es,It,Nl) (Rev 1) | ASIE | Eeprom | ok |  |
| Sister Princess - RePure (Japan) | A4PJ | Eeprom | ok |  |
| Sitting Ducks (USA) (En,Fr,De,Es,It,Nl) | BSDE | None | ok |  |
| Sky Dancers - They Magically Fly! (USA) | B4DE | None | ok |  |
| Slime Morimori Dragon Quest - Shougeki no Shippo Dan (Japan) | A9KJ | Eeprom | ok |  |
| Slot! Pro 2 Advance - GoGo Juggler & New Tairyou (Japan) | ATBJ | Eeprom | ok |  |
| Slot! Pro Advance - Takarabune & Ooedo Sakurafubuki 2 (Japan) | ASFJ | Eeprom | ok |  |
| Smashing Drive (USA) | BSVE | None | ok |  |
| Smuggler's Run (USA) | ASGE | None | ok |  |
| Snap Kid's (Japan) | AEAJ | Sram32K | ok |  |
| Snood (USA) | ASQE | None | ok |  |
| Snood 2 - On Vacation (USA) | B2VE | None | ok |  |
| Soccer Kid (USA, Europe) | AK6E | None | ok |  |
| Soccer Mania (USA, Europe) (En,Fr,De,Es,It,Nl,Sv,Da) | ALSE | Eeprom | ok |  |
| Sonic Advance (USA) (En,Ja) | ASOE | Flash64K | ok |  |
| Sonic Advance 2 (USA) (En,Ja,Fr,De,Es,It) | A2NE | Flash64K | waiting | REG. NAME entry of a blank save, waiting for input |
| Sonic Advance 3 (USA) (En,Ja,Fr,De,Es,It) | B3SE | Flash64K | ok |  |
| Sonic Battle (USA) (En,Ja,Fr,De,Es,It) | BSBE | Flash64K | ok |  |
| Sonic Pinball Party (USA) (En,Ja,Fr,De,Es,It) | A3VE | Flash64K | ok |  |
| Sonic The Hedgehog - Genesis (USA) | BIJE | Eeprom | ok |  |
| Sound of Thunder, A (USA) (En,Fr,De,Es,It) | A3QE | None | ok |  |
| Space Channel 5 - Ulala's Cosmic Attack (USA) | A5UE | Eeprom | ok |  |
| Space Hexcite - Maetel Legend EX (Japan) | AJSJ | Sram32K | ok |  |
| Space Invaders (USA, Europe) | AIDE | Eeprom | ok |  |
| Speedball 2 (Europe) (En,Fr,De,Es,It) | AS6P | Eeprom | ok |  |
| Spider-Man (USA, Europe) | AKXE | Eeprom | ok |  |
| Spider-Man - Battle for New York (USA) | BC9E | Eeprom | ok |  |
| Spider-Man - Mysterio's Menace (USA, Europe) | ASEE | None | ok |  |
| Spider-Man 2 (USA, Europe) | BSPE | Eeprom | ok |  |
| Spider-Man 3 (USA) | BI3E | Eeprom | ok |  |
| Spirit - Stallion of the Cimarron - Search for Homeland (USA) | AC6E | None | ok |  |
| Spirits & Spells (USA) | AWNE | None | ok |  |
| SpongeBob SquarePants - Battle for Bikini Bottom (USA) | BSQE | None | ok |  |
| SpongeBob SquarePants - Creature from the Krusty Krab (USA) | BQ4E | Eeprom | ok |  |
| SpongeBob SquarePants - Lights, Camera, Pants! (USA) | BQQE | Eeprom | ok |  |
| SpongeBob SquarePants - Revenge of the Flying Dutchman (USA, Europe) | AQ3E | None | ok |  |
| SpongeBob SquarePants - SuperSponge (USA, Europe) | ASPE | None | ok |  |
| SpongeBob SquarePants Movie, The (USA) | BSNE | None | ok |  |
| SpongeBob's Atlantis SquarePantis (USA) | BZXE | None | ok |  |
| Sports Illustrated for Kids - Baseball (USA) | AKBE | Sram32K | waiting | menu waiting for input (Pick a state) |
| Sports Illustrated for Kids - Football (USA) | AKFE | Sram32K | waiting | menu waiting for input (Pick a state) |
| Spy Hunter (USA) (En,Ja,Fr,De,Es) | AHNE | Eeprom | ok |  |
| Spy Kids 3-D - Game Over (USA) | AV3E | Eeprom | ok |  |
| Spy Kids Challenger (USA) | A2KE | Eeprom | ok |  |
| Spy Muppets - License to Croak (USA) (En,Fr,De,Es,It,Nl) | BSSE | None | ok |  |
| Spyro - Attack of the Rhynocs (USA) | AOWE | Eeprom | ok |  |
| Spyro - Season of Ice (USA) | ASYE | Eeprom | ok |  |
| Spyro 2 - Season of Flame (USA) | A2SE | Eeprom | ok |  |
| Spyro Orange - The Cortex Conspiracy (USA) (Rev 1) | BSTE | Eeprom | ok |  |
| SSX 3 (USA, Europe) | BSXE | Eeprom | ok |  |
| SSX Tricky (USA, Europe) (En,Fr,De) | AXYE | Eeprom | ok |  |
| Stadium Games (USA) | A9GE | None | ok |  |
| Star Wars - Episode II - Attack of the Clones (USA) | AS2E | None | ok |  |
| Star Wars - Episode III - Revenge of the Sith (USA) (En,Fr,Es) | BE3E | Eeprom | ok |  |
| Star Wars - Flight of the Falcon (USA) | BSWE | None | ok |  |
| Star Wars - Jedi Power Battles (USA) | ASWE | None | ok |  |
| Star Wars - The New Droid Army (USA) | A2WE | None | ok |  |
| Star Wars Trilogy - Apprentice of the Force (USA) (En,Fr,Es) | BCKE | Eeprom | ok |  |
| Star X (USA) (En,Fr,De,Es,It,Nl) | AS8E | None | ok |  |
| Starsky & Hutch (USA) | AYHE | Eeprom | ok |  |
| Steel Empire (Europe) | BKTP | Eeprom | ok |  |
| Strawberry Shortcake - Ice Cream Island - Riding Camp (Europe) (En,Fr,De,Es,It,Nl,Pt,Da) (Rev 1) | B35P | Eeprom | ok |  |
| Strawberry Shortcake - Summertime Adventure (USA) | B35E | None | ok |  |
| Strawberry Shortcake - Summertime Adventure - Special Edition (USA) | BQWE | None | ok |  |
| Strawberry Shortcake - Sweet Dreams (USA) | B4TE | Eeprom | ok |  |
| Street Fighter Alpha 3 (USA) | AZUE | Eeprom | ok |  |
| Street Jam Basketball (USA, Europe) | A3ZE | Eeprom | ok |  |
| Street Racing Syndicate (USA) | BCZE | Eeprom | ok |  |
| Strike Force Hydra (USA) | AFHE | None | ok |  |
| Stuart Little 2 (USA, Europe) | ASLE | None | ok |  |
| Stuntman (USA) (En,Fr,Es) | AUXE | Eeprom | ok |  |
| Sugar Sugar Rune - Heart Ga Ippai! Moegi Gakuen (Japan) | B4LJ | Eeprom | ok |  |
| Suite Life of Zack & Cody, The - Tipton Caper (USA) (En,Fr) | BZCE | Eeprom | ok |  |
| Sum of All Fears, The (USA) (En,Fr,De,Es,It) | AA6E | Eeprom | ok |  |
| Summon Night - Craft Sword Monogatari - Hajimari no Ishi (Japan) | B3CJ | Eeprom | waiting | menu waiting for input (hero select) |
| Summon Night - Swordcraft Story (USA) | AB4E | Eeprom | ok |  |
| Summon Night - Swordcraft Story 2 (USA) | BSKE | Eeprom | ok |  |
| Super Army War (USA) | BG6E | None | ok |  |
| Super Bubble Pop (USA) | AVZE | Eeprom | ok |  |
| Super Bust-A-Move (USA) (En,Fr,Es) | ABME | None | ok |  |
| Super Chinese 1, 2 Advance (Japan) | BSAJ | Eeprom | ok |  |
| Super Collapse! II (USA) | BCLE | None | ok |  |
| Super Dodge Ball Advance (USA) (Rev 1) | ADFE | Sram32K | ok |  |
| Super Dropzone - Intergalactic Rescue Mission (USA) | AZNE | None | ok |  |
| Super Duper Sumos (USA) | BDPE | Eeprom | ok |  |
| Super Ghouls'n Ghosts (USA, Europe) | AG5E | Eeprom | ok |  |
| Super Hornet FA 18F (USA, Europe) | BF8E | None | ok |  |
| Super Mario Advance (USA, Europe) | AMAE | Eeprom | ok |  |
| Super Mario Advance 2 - Super Mario World (USA, Australia) | AA2E | Eeprom | ok |  |
| Super Mario Advance 3 - Yoshi's Island (USA) | A3AE | Eeprom | ok |  |
| Super Mario Advance 4 - Super Mario Bros. 3 (USA, Australia) (Rev 1) | AX4E | Flash128K | ok |  |
| Super Monkey Ball Jr. (USA) | ALUE | Eeprom | ok |  |
| Super Puzzle Fighter II (USA) (Rev 1) | AZ8E | Eeprom | ok |  |
| Super Real Mahjong Dousoukai (Japan) | BDMJ | Eeprom | ok |  |
| Super Robot Taisen - Original Generation (USA) | AOGE | Flash64K | ok |  |
| Super Robot Taisen - Original Generation 2 (USA) | B2RE | Flash64K | ok |  |
| Super Robot Taisen A (Japan) | ASRJ | Flash64K | ok |  |
| Super Robot Taisen D (Japan) | A6SJ | Flash64K | ok |  |
| Super Robot Taisen J (Japan) | B6JJ | Flash64K | ok |  |
| Super Robot Taisen R (Japan) | AJ9J | Flash64K | ok |  |
| Super Street Fighter II Turbo - Revival (USA) | AXRE | Eeprom | ok |  |
| Superman - Countdown to Apokolips (USA) | ASUE | None | ok |  |
| Superman Returns - Fortress of Solitude (USA, Europe) (En,Fr,De,Es,It) | BQXE | Eeprom | ok |  |
| Surf's Up (USA) (En,Fr,Es) | BXUE | Eeprom | ok |  |
| Sutakomi - Star Communicator (Japan) | ASKJ | Sram32K | ok |  |
| Sweet Cookie Pie (Japan) | ABGJ | Eeprom | ok |  |
| Sword of Mana (USA, Australia) | AVSE | Flash64K | ok |  |
| Sylvanian Families - Fashion Designer ni Naritai! - Kurumi-risu no Onnanoko (Japan) | BSFJ | Eeprom | ok |  |
| Sylvanian Families - Yousei no Stick to Fushigi no Ki - Marron-inu no Onnanoko (Japan) | BS5J | Eeprom | ok |  |
| Sylvanian Families 4 - Meguru Kisetsu no Tapestry (Japan) | A4LJ | Eeprom | ok |  |
| Tactics Ogre - The Knight of Lodis (USA) | ATOE | Flash64K | ok |  |
| Taiketsu! Ultra Hero (Japan) | BU6J | Eeprom | ok |  |
| Tak - The Great Juju Challenge (USA, Europe) | BJWE | Eeprom | ok |  |
| Tak 2 - The Staff of Dreams (USA) | BT9E | Eeprom | ok |  |
| Tak and the Power of Juju (USA) | BJUE | Eeprom | ok |  |
| Tales of Phantasia (USA, Australia) | AN8E | Eeprom | ok |  |
| Tales of the World - Narikiri Dungeon 2 (Japan) | AN9J | Eeprom | ok |  |
| Tales of the World - Narikiri Dungeon 3 (Japan) | B3TJ | Eeprom | ok |  |
| Tales of the World - Summoner's Lineage (Japan) | A9PJ | Sram32K | ok |  |
| Tanbi Musou - Meine Liebe (Japan) | AYMJ | Sram32K | ok |  |
| Tang Tang (USA) | ATAE | Eeprom | ok |  |
| Tantei Gakuen Q - Kyuukyoku Trick ni Idome! (Japan) | BTIJ | Sram32K | ok |  |
| Tantei Gakuen Q - Meitantei wa Kimi da! (Japan) | BTQJ | Sram32K | ok |  |
| Tantei Jinguuji Saburou - Shiroi Kage no Shoujo (Japan) | BT3J | Eeprom | ok |  |
| Tarzan - Return to the Jungle (USA, Europe) | AJGE | Eeprom | ok |  |
| Taxi 3 (France) | AXQF | None | ok |  |
| Teen Titans (USA) (En,Fr) | BBLE | Eeprom | ok |  |
| Teen Titans 2 (USA) (En,Fr) | BZUE | Eeprom | ok |  |
| Teenage Mutant Ninja Turtles (USA) | BNTE | Eeprom | ok |  |
| Teenage Mutant Ninja Turtles 2 - Battle Nexus (USA) | BT2E | Eeprom | ok |  |
| Tekken Advance (USA) | ATKE | Eeprom | ok |  |
| Ten Pin Alley 2 (USA) | BTPE | None | ok |  |
| Tennis Masters Series 2003 (Europe) (En,Fr,De,Es,It,Pt) | AT8P | Eeprom | ok |  |
| Tennis no Ouji-sama - Aim at the Victory! (Japan) | AVAJ | Sram32K | ok |  |
| Tennis no Ouji-sama - Genius Boys Academy (Japan) | ATIJ | Flash64K | ok |  |
| Tennis no Ouji-sama 2003 - Cool Blue (Japan) | A9LJ | Sram32K | ok |  |
| Tennis no Ouji-sama 2003 - Passion Red (Japan) | A8RJ | Sram32K | ok |  |
| Tennis no Ouji-sama 2004 - Glorious Gold (Japan) | B4GJ | Sram32K | black | OPEN: an uninitialised local in the game's init code corrupts its IWRAM sound mixer; outcome depends on IRQ phase (see PROGRESS.md) |
| Tennis no Ouji-sama 2004 - Stylish Silver (Japan) | B4SJ | Sram32K | black | OPEN: same as Glorious Gold |
| Terminator 3 - Rise of the Machines (USA) | AO3E | None | ok |  |
| Tetris Worlds (USA) | ATWE | None | ok |  |
| Texas Hold 'em Poker (USA) | BXAE | None | ok |  |
| That's So Raven (USA) | BRVE | Eeprom | ok |  |
| That's So Raven 2 - Supernatural Style (USA) (En,Fr) | BZSE | Eeprom | ok |  |
| Three Stooges, The (USA) | A3TE | None | ok |  |
| Three-in-One Pack - Connect Four + Perfection + Trouble (USA) | B65E | None | ok |  |
| Three-in-One Pack - Risk + Battleship + Clue (USA) (Rev 1) | B66E | None | ok |  |
| Three-in-One Pack - Sorry! + Aggravation + Scrabble Junior (USA) | B67E | None | ok |  |
| Thunder Alley (USA) | BTHE | None | ok |  |
| Thunderbirds (USA, Europe) | BTBE | None | ok |  |
| Thunderbirds - International Rescue (Europe) | ATNP | None | ok |  |
| Tiger Woods PGA Tour 2004 (USA, Europe) | BTWE | Eeprom | ok |  |
| Tiger Woods PGA Tour Golf (USA, Europe) | AT5E | Eeprom | ok |  |
| Tim Burton's The Nightmare Before Christmas - The Pumpkin King (USA, Europe) (En,Fr,De,Es,It) | BNCE | Eeprom | ok |  |
| Tiny Toon Adventures - Scary Dreams (USA) | ATTE | None | ok |  |
| Tiny Toon Adventures - Wacky Stackers (USA) | AWSE | None | ok |  |
| Tir et But - Edition Champions du Monde (France) | ATVP | None | ok |  |
| Titeuf - Mega Compet (France) | BTCF | None | ok |  |
| Titeuf - Ze Gag Machine (France) | AT7F | None | ok |  |
| TMNT (USA) (En,Fr,Es) | BEXE | Eeprom | ok |  |
| TOCA World Touring Cars (Europe) | ATQP | Eeprom | ok |  |
| Tokimeki Yume Series 1 - Ohanaya-san ni Narou! (Japan) | AF7J | Eeprom | ok |  |
| Tokyo Majin Gakuen - Fuju Houroku (Japan) | BTFJ | Sram32K | ok |  |
| Tokyo Xtreme Racer Advance (USA) | BTZE | Eeprom | ok |  |
| Tom and Jerry - The Magic Ring (USA) (En,Fr,De,Es,It) | ATJE | None | ok |  |
| Tom and Jerry in Infurnal Escape (USA) | AIFE | None | ok |  |
| Tom and Jerry Tales (USA) (En,Fr,Es) | BJTE | None | ok |  |
| Tom Clancy's Rainbow Six - Rogue Spear (USA) (En,Fr,De,Es,It) | AR6E | Eeprom | ok |  |
| Tom Clancy's Splinter Cell (USA) (En,Fr,Es) | AO4E | Eeprom | ok |  |
| Tom Clancy's Splinter Cell - Pandora Tomorrow (USA) (En,Fr,Es) | BSLE | Eeprom | ok |  |
| Tomato Adventure (Japan) | AGLJ | Eeprom | ok |  |
| Tonka - On the Job (USA) | BT7E | None | ok |  |
| Tony Hawk's American Sk8land (USA) | BH9E | Eeprom | ok |  |
| Tony Hawk's Downhill Jam (USA) | BXSE | Eeprom | ok |  |
| Tony Hawk's Pro Skater 2 (USA, Europe) | ATHE | Eeprom | ok |  |
| Tony Hawk's Pro Skater 3 (USA, Europe) | AT3E | Eeprom | ok |  |
| Tony Hawk's Pro Skater 4 (USA, Europe) | AT6E | Eeprom | ok |  |
| Tony Hawk's Underground (USA, Europe) | BTOE | Eeprom | ok |  |
| Tony Hawk's Underground 2 (USA, Europe) | B2TE | Eeprom | ok |  |
| Top Gear GT Championship (USA) | ATCE | Eeprom | ok |  |
| Top Gear Rally (USA) | BTGE | Eeprom | ok |  |
| Top Gun - Combat Zones (USA) (En,Fr,De,Es,It) | A2YE | Flash64K | waiting | main menu waiting for input |
| Top Gun - Firestorm Advance (USA, Europe) (En,Fr,De,Es,It) | ATGE | None | ok |  |
| Top Spin 2 (USA) (En,Fr,De,Es,It) | B27E | Eeprom | ok |  |
| Total Soccer 2002 (Europe) (En,Fr,De,Es,It,Nl) | ATUP | Eeprom | ok |  |
| Total Soccer Manager (Europe) (En,Fr,De,Es,It,Nl) | ATFP | Eeprom | ok |  |
| Totally Spies! (USA) | BTUE | Eeprom | ok |  |
| Totally Spies! 2 - Undercover (USA) (En,Fr) | B2LE | Eeprom | ok |  |
| Touhai Densetsu Akagi - Yami ni Mai Orita Tensai (Japan) | BZWJ | Eeprom | ok |  |
| Toukon Heat (Japan) | A59J | Eeprom | ok |  |
| Tower SP, The (USA) | BTRE | Flash64K | ok |  |
| Toy Robo Force (Japan) | ATRJ | Sram32K | ok |  |
| Treasure Planet (USA) | AZQE | None | ok |  |
| Trick Star (Europe) (En,Fr,De,Es,It) | B9SP | Eeprom | ok |  |
| Tringo (USA) | BTJE | Eeprom | ok |  |
| Trollz - Hair Affair! (USA) | BT6E | Eeprom | ok |  |
| Tron 2.0 - Killer App (USA) | BTNE | Eeprom | ok |  |
| Turbo Turtle Adventure (USA) | AK3E | Eeprom | ok |  |
| Turok - Evolution (USA) | AT4E | None | ok |  |
| Tweety and the Magic Gems (USA) | ATME | Sram32K | ok |  |
| Twin Series 1 - Mezase Debut! - Fashion Designer Monogatari + Kawaii Pet Game Gallery 2 (Japan) | BFVJ | Eeprom | ok |  |
| Twin Series 2 - Oshare Princess 4 + Renai Uranai Daisakusen! + Renai Party Game - Sweet Heart (Japan) | BOPJ | Eeprom | ok |  |
| Twin Series 3 - Konchuu Monster - Ouja Ketteisen + Super Chinese Labyrinth (Japan) | BQMJ | Eeprom | ok |  |
| Twin Series 4 - Hamu Hamu Monster EX - Hamster Monogatari RPG + Fantasy Puzzle - Hamster Monogatari - Mahou no Meikyuu 1.2.3 (Japan) | BHFJ | Eeprom | ok |  |
| Twin Series 5 - Mahou no Kuni no Cake-ya-san Monogatari + Wanwan Meitantei EX (Japan) | BMWJ | Eeprom | ok |  |
| Twin Series 6 - Wannyan Idol Gakuen + Koinu to Issho Special (Japan) | BWNJ | Eeprom | ok |  |
| Twin Series 7 - Twin Puzzle - Kisekae Wanko EX + Nyaa to Chuu no Rainbow Magic 2 (Japan) | B2PJ | Eeprom | ok |  |
| Ty the Tasmanian Tiger 2 - Bush Rescue (USA, Europe) (En,Fr,De) | BTYE | Eeprom | ok |  |
| Ty the Tasmanian Tiger 3 - Night of the Quinkan (USA) | BTVE | Eeprom | ok |  |
| Uchuu Daisakusen Choco Vader - Uchuu kara no Shinryakusha (Japan) | AUCJ | Eeprom | ok |  |
| Uchuu no Stellvia (Japan) | BUVJ | Sram32K | ok |  |
| Ueki no Housoku - Jingi Sakuretsu! Nouryokusha Battle (Japan) | BUHJ | Eeprom | ok |  |
| Ui-Ire - World Soccer Winning Eleven (Japan) | AEWJ | Eeprom | ok |  |
| Ultimate Arcade Games (USA) | BUZE | Eeprom | ok |  |
| Ultimate Beach Soccer (USA) | AVEE | Eeprom | ok |  |
| Ultimate Brain Games (USA, Europe) | ABUE | Eeprom | ok |  |
| Ultimate Card Games (USA) (Rev 2) | BUCE | Eeprom | ok |  |
| Ultimate Muscle - The Kinnikuman Legacy - The Path of the Superhero (USA) | AK2E | Eeprom | ok |  |
| Ultimate Puzzle Games (USA) | BUAE | Eeprom | ok |  |
| Ultimate Spider-Man (USA) | BULE | Eeprom | ok |  |
| Ultimate Winter Games (USA) | BUWE | None | ok |  |
| Ultra Keibitai - Monster Attack (Japan) | BUTJ | Flash64K | ok |  |
| Unfabulous (USA) | BU4E | None | ok |  |
| Uno - Free Fall (USA) | BUIE | None | ok |  |
| Uno 52 (USA) | BU5E | None | ok |  |
| Urban Yeti! (USA, Europe) | AYIE | None | ok |  |
| Urbz, The - Sims in the City (USA, Europe) (En,Fr,De,Es,It,Nl) | BOCE | Flash64K | ok |  |
| V-Master Cross (Japan) | AVMJ | Eeprom | ok |  |
| V-Rally 3 (USA) (En,Fr,Es) | AVRE | Eeprom | ok |  |
| V.I.P. (Europe) (En,Fr,De,Es,It,Nl) | AVPP | None | ok |  |
| Van Helsing (USA) | BANE | None | ok |  |
| Vattroller X (Japan) | BRXJ | Sram32K | ok |  |
| VeggieTales - LarryBoy and the Bad Apple (USA) | BZTE | Eeprom | ok |  |
| Virtua Tennis (USA) | AVTE | Eeprom | ok |  |
| Virtual Kasparov (USA) (En,Fr,De,Es,It) | AVKE | Eeprom | ok |  |
| W.i.t.c.h. (Europe) (En,Fr,De,Es,It) | BWTP | Eeprom | ok |  |
| Wade Hixton's Counter Punch (USA, Europe) | BSRE | Eeprom | ok |  |
| Wagamama Fairy Mirumo de Pon! - 8 Nin no Toki no Yousei (Japan) | BMYJ | Sram32K | ok |  |
| Wagamama Fairy Mirumo de Pon! - Dokidoki Memorial Panic (Japan) | BMIJ | Sram32K | ok |  |
| Wagamama Fairy Mirumo de Pon! - Nazo no Kagi to Shinjitsu no Tobira (Japan) | BWPJ | Sram32K | ok |  |
| Wagamama Fairy Mirumo de Pon! - Ougon Maracas no Densetsu (Japan) | AWKJ | Eeprom | ok |  |
| Wagamama Fairy Mirumo de Pon! - Taisen Mahoudama (Japan) | BMPJ | Sram32K | ok |  |
| Wagamama Fairy Mirumo de Pon! - Yume no Kakera (Japan) | BWFJ | Sram32K | ok |  |
| Wakeboarding Unleashed Featuring Shaun Murray (USA) | AWDE | Eeprom | ok |  |
| Wanko de Kururin! Wancle (Japan) | BWKJ | Eeprom | ok |  |
| Wanko Mix Chiwanko World (Japan) | BWXJ | Eeprom | ok |  |
| Wannyan Doubutsu Byouin - Doubutsu no Oishasan Ikusei Game (Japan) (Rev 1) | BWDJ | Eeprom | ok |  |
| Wanwan Meitantei (Japan) | BWMJ | Eeprom | ok |  |
| Wario Land 4 (USA, Europe) | AWAE | Sram32K | ok |  |
| WarioWare - Twisted! (USA, Australia) | RZWE | Sram32K | ok |  |
| WarioWare, Inc. - Mega Microgame$! (USA) | AZWE | Sram32K | ok |  |
| Whac-A-Mole (USA) | BWEE | Eeprom | ok |  |
| Whistle! - Dai-37-kai Tokyo-to Chuugakkou Sougou Taiiku Soccer Taikai (Japan) | A73J | Sram32K | ok |  |
| Who Wants to Be a Millionaire (Europe) | A55P | None | ok |  |
| Who Wants to Be a Millionaire - 2nd Edition (Europe) | B55P | None | ok |  |
| Who Wants to Be a Millionaire Junior (Europe) | BWJP | None | ok |  |
| Wild Thornberrys Movie, The (USA, Europe) | AWLE | None | ok |  |
| Wild Thornberrys, The - Chimp Chase (USA, Europe) | AWTE | None | ok |  |
| Wild, The (USA, Europe) (En,Fr,De,Es) | BWLE | Eeprom | ok |  |
| Wilden Fussball-Kerle, Die - Entscheidung im Teufelstopf (Germany) | BWUD | None | ok |  |
| Wilden Fussball-Kerle, Die - Gefahr im Wilde Kerle Land (Germany) | BXWD | Eeprom | ok |  |
| Wing Commander - Prophecy (USA) | AW9E | Eeprom | ok |  |
| Wings (USA) | AWQE | Eeprom | ok |  |
| Winnie the Pooh's Rumbly Tumbly Adventure (USA) (En,Fr,Es) | BWHE | None | ok |  |
| Winning Post for Game Boy Advance (Japan) (Rev 2) | AWPJ | Flash64K | ok |  |
| Winter Sports (Europe) (En,Fr,De,Es,It) | BWYP | Eeprom | ok |  |
| WinX Club (USA) | BWIE | Eeprom | ok |  |
| WinX Club - Quest for the Codex (USA) | BWVE | Eeprom | ok |  |
| Wizardry Summoner (Japan) (Rev 1) | AWZJ | Sram32K | ok |  |
| Wolfenstein 3D (USA, Europe) | AWOE | Eeprom | ok |  |
| Woody Woodpecker in Crazy Castle 5 (USA) | AWWE | Eeprom | ok |  |
| Word Safari - The Friendship Totems (USA) | BB8E | Eeprom | ok |  |
| World Advance Soccer - Shouri e no Michi (Japan) | AASJ | Eeprom | ok |  |
| World Championship Poker (USA) | BP9E | None | ok |  |
| World Poker Tour (USA) | B26E | Eeprom | ok |  |
| World Tennis Stars (USA) | AWCE | None | ok |  |
| Worms - World Party (USA) (En,Fr,De,Es,It) | AWYE | None | ok |  |
| Worms Blast (Europe) (En,Fr,De,Es,It) | AWBP | None | ok |  |
| WTA Tour Tennis (USA) | ATEE | Eeprom | ok |  |
| WWE - Road to WrestleMania X8 (USA, Europe) | AW8E | Eeprom | ok |  |
| WWE - Survivor Series (USA, Europe) | BWWE | Eeprom | ok |  |
| WWF - Road to WrestleMania (USA, Europe) | AWFE | None | ok |  |
| X-Bladez - Inline Skater (USA) | AXIE | None | ok |  |
| X-Men - Reign of Apocalypse (USA, Europe) | AXME | Eeprom | ok | music through the HLE BIOS sound driver (2026-10-01); pinned in tests/compat.toml |
| X-Men - The Official Game (USA) | B3XE | Eeprom | ok |  |
| X2 - Wolverine's Revenge (USA, Europe) | AWVE | Eeprom | ok |  |
| XS Moto (USA) | BXME | None | ok |  |
| xXx (USA, Europe) | AX3E | Eeprom | ok |  |
| Yggdra Union - We'll Never Fight Alone (USA) | BYUE | Eeprom | ok |  |
| Yoshi Topsy-Turvy (USA) | KYGE | Eeprom | ok |  |
| Youkaidou (Japan) | AFUJ | Eeprom | ok |  |
| Yu Yu Hakusho - Ghostfiles - Spirit Detective (USA) | BYYE | Eeprom | ok |  |
| Yu Yu Hakusho - Ghostfiles - Tournament Tactics (USA, Europe) | BRGE | Eeprom | ok |  |
| Yu-Gi-Oh! - 7 Trials to Glory - World Championship Tournament 2005 (USA) (En,Ja,Fr,De,Es,It) | BY7E | Sram32K | ok |  |
| Yu-Gi-Oh! - Destiny Board Traveler (USA) | BYDE | Sram32K | ok |  |
| Yu-Gi-Oh! - Dungeon Dice Monsters (USA) (En,Es) | AYDE | Flash64K | ok |  |
| Yu-Gi-Oh! - Reshef of Destruction (USA) | AY8E | Sram32K | ok |  |
| Yu-Gi-Oh! - The Eternal Duelist Soul (USA) | AY5E | Sram32K | ok |  |
| Yu-Gi-Oh! - The Sacred Cards (USA) | AY7E | Sram32K | ok |  |
| Yu-Gi-Oh! - Ultimate Masters - World Championship Tournament 2006 (USA) (En,Ja,Fr,De,Es,It) | BY6E | Sram32K | ok |  |
| Yu-Gi-Oh! - World Championship Tournament 2004 (USA) (En,Ja,Fr,De,Es,It) | BYWE | Sram32K | ok |  |
| Yu-Gi-Oh! - Worldwide Edition - Stairway to the Destined Duel (USA) (En,Ja,Fr,De,Es,It) | AYWE | Sram32K | ok |  |
| Yu-Gi-Oh! Double Pack (USA) | BY2E | Sram32K | ok |  |
| Yu-Gi-Oh! Double Pack 2 (USA) (En,Fr,De,Es,It) | BYVE | Sram32K | ok |  |
| Yu-Gi-Oh! Duel Monsters 6 Expert 2 (Japan) | AY6J | Sram32K | ok |  |
| Yu-Gi-Oh! GX - Duel Academy (USA) | BYGE | Sram32K | ok |  |
| Yumemi-chan no Naritai Series 3 - Watashi no Makesalon (Japan) | AW3J | Eeprom | ok |  |
| Yuujou no Victory Goal 4v4 Arashi - Get the Goal!! (Japan) | A4VJ | Sram32K | ok |  |
| Yuureiyashiki no Nijuuyojikan (Japan) | AUYJ | Eeprom | ok |  |
| Zapper - One Wicked Cricket! (USA) | AZPE | Eeprom | ok |  |
| Zatchbell! - Electric Arena (USA) | A4GE | Eeprom | ok |  |
| Zen-Nihon Shounen Soccer Taikai 2 - Mezase Nihon-ichi! (Japan) | A2ZJ | Flash64K | ok |  |
| Zero One (Japan) | AF3J | Eeprom | ok |  |
| Zero One SP (Japan) | BZOJ | Eeprom | ok |  |
| Zero-Tours (Japan) | AZTJ | Flash64K | ok |  |
| Zettaizetsumei Dangerous Jiisan - Naki no 1-kai - Zettaifukujuu Violence Kouchou - Wagahai ga 1-ban Erainjai!! (Japan) | BZGJ | Sram32K | ok |  |
| Zettaizetsumei Dangerous Jiisan - Shijou Saikyou no Dogeza (Japan) | BZDJ | Sram32K | ok |  |
| Zettaizetsumei Dangerous Jiisan 3 - Hateshinaki Mamonogatari (Japan) | BJ3J | Sram32K | ok |  |
| Zettaizetsumei Dangerous Jiisan Tsuu - Ikari no Oshioki Blues (Japan) | BZ2J | Sram32K | ok |  |
| Zidane - Football Generation 2002 (Europe) (En,Fr,De,Es,It) | AZDP | None | ok |  |
| Zoey 101 (USA) | BZYE | None | ok |  |
| Zoids - Legacy (USA) | AZ2E | Sram32K | ok |  |
| Zoids Saga (Japan) (Rev 1) | ATZJ | Sram32K | ok |  |
| Zoids Saga - Fuzors (Japan) | BZFJ | Sram32K | ok |  |
| Zone of the Enders - The Fist of Mars (USA) | AZEE | Flash64K | ok |  |
| ZooCube (USA) | ANCE | Eeprom | ok |  |
| Zooo - Action Puzzle Game (Europe) (En,Fr,De,Es,It) | BMZP | Eeprom | ok |  |
