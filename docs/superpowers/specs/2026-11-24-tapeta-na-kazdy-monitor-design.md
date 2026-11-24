# Tapeta na každý monitor — návrh

24. 11. 2026. Domluveno s uživatelem v rozhovoru téhož dne. Co je
**[moje volba]**, navrhl stavitel; ostatní řekl, vybral nebo odsouhlasil
uživatel v rozhovoru.

## Záměr

Dnes WallFlux hraje jednu tapetu a na další monitory ji jen zrcadlí (režim
Wallpaper Engine „Clone single wallpaper“). Uživatel chce, aby každý monitor
mohl mít **svou tapetu se svým zvukem** — tak jak to Wallpaper Engine dělá
ve výchozím stavu („Wallpaper per display“). Zásada: **kopírovat Wallpaper
Engine celý, nevymýšlet znovu** — kromě roztažení jedné tapety přes monitory
a dělení monitoru, které uživatel výslovně vyřadil.

Chování Wallpaper Engine bylo přečteno z jeho vlastního rozhraní
(`ui/dist/scripts/scripts.js`, `locale/ui_en-us.json`,
`locale/core_en-us.json`): rozložení, výběr monitoru, kontextová nabídka
monitoru, ztlumení po monitoru, playlist a vlastnosti po monitoru, pauza „po
monitoru“ vedle „vše“, profily.

## Cíle

- Dvě rozložení: **Tapeta na každý monitor** (výchozí) a **Klonovat jednu
  tapetu** (dnešní chování).
- Po monitoru: tapeta, playlist, vlastnosti tapety, ztlumení, pauza z traye,
  přechody.
- Zvuk ze všech monitorů, které nejsou ztlumené, naráz.
- Tray s nabídkou pro každý monitor.
- Profily: celé rozestavení monitorů uložené pod jménem.

## Ne-cíle

- Roztažení jedné tapety přes víc monitorů (*Stretch*), dělení monitoru
  (*Split*), skupiny monitorů (*Stretch/Clone group*) **[moje volba]**.
- Spořič obrazovky, mobilní zařízení, klávesové zkratky (Wallpaper Engine je
  má u monitorů a profilů; tady se nedělají, viz CLAUDE.md o zkratkách).
- Kvalita po monitoru — zůstává globální a po tapetě, jako ve WE.

## Anti-cíle (co by bylo selháním, i kdyby to „fungovalo“)

- Rozbitý klon: režim Klonovat musí po změně dělat přesně to, co dnes
  (jedna tapeta spočítaná jednou, ostatní obrazovky zrcadlí; jeden zvuk).
- Něco, co se samo spustí po startu programu. Pravidlo „nic nehraje při
  startu“ platí dál (viz CLAUDE.md, Decisions); profily ho neporušují, protože
  se nahrávají jen na pokyn.
- S jedním monitorem jakákoli změna v tom, co uživatel vidí: žádné „Monitor 1“,
  žádný výběr monitoru, tray jako dnes.
- Na notebooku (A8-6410) zahlcený stroj: dvě scény naráz se musí změřit, ne
  předpokládat. Žádná automatická brzda se nestaví — Wallpaper Engine žádnou
  nemá, těžká scéna u něj prostě dostane méně snímků, a uživatel má předvolbu
  Nízká. Změří se a výsledek se uživateli řekne; vyjde-li to nepoužitelně,
  probere se zvlášť **[moje volba]**.

## Omezení

- C++20, Qt 6, QML; renderer v Rustu se nemění (scénu na každé obrazovce
  zvlášť už uměl do 6. 11.).
- Všechny pasti v CLAUDE.md platí — hlavně: vazba na JS funkci se neobnoví
  bez argumentu, který čte; `select()` nesmí hlásit změnu, když se nic
  nezměnilo; `Video.autoPlay` / `onSourceChanged: play()`; zvuk vlastního
  streamu se pozná podle `node.name` `wallflux`; layer-shell plocha musí znát
  svou obrazovku (`Desktop::asWallpaper`).
- Obrazovky jsou jen Qt (`Application.screens`), pojmenované podle konektoru.
- Čeština a angličtina; texty Wallpaper Engine se nepřebírají z jeho
  `locale/` (to je jen pro štítky dílny), píší se do `wallflux_cs.ts`.

## Přístupy

1. **Každý monitor vlastní „přehrávač“** — vybráno. Dnešní jedna sada toho, co
   hraje (tapeta, běžící playlist, pauza, přechody, hudba scény), se stane
   objektem, kterého je tolik, kolik je monitorů. Klon = jeden přehrávač pro
   všechny obrazovky, což je dnešní program. Oba režimy na stejném kódu.
   Cena: velký zásah do `Main.qml`, rozdělení `Playlists`, knihovna a tray se
   ptají na vybraný monitor místo na `Wallpapers.current`.
2. **Jedna sada s tabulkami „na kterém monitoru“** — každý singleton by nesl
   mapu podle jména obrazovky a každé volání jméno. Zamítnuto: vazby na funkce
   s parametrem se v QML samy neobnovují (past v CLAUDE.md), chyb by bylo víc
   a hůř hledatelných.
3. **Kopie programu pro každý monitor** — zamítnuto hned: dvě ikony v trayi,
   dvě okna, a renderer je do programu slinkovaný záměrně.

## Chování (rozhodnuto s uživatelem)

### Detekce a jeden monitor

- Program pozná počet připojených monitorů. **S jedním monitorem** (nebo s
  víc monitory v režimu Klonovat) vypadá knihovna i tray jako dnes.

### Karta Monitory

- Nahoře volba rozložení: *Tapeta na každý monitor* / *Klonovat jednu tapetu*.
- Nákres monitorů podle jejich skutečného rozestavení zůstává. Na každém
  monitoru náhled jeho tapety a ikonka reproduktoru (klik = ztlumit /
  zrušit ztlumení). Klik na monitor ho vybere.
- **Pravý klik na monitor**: *Změnit tapetu* (otevře knihovnu s tímto
  monitorem vybraným), *Odebrat tapetu*, *Kopírovat na…* ▸ (ostatní
  monitory), *Prohodit s…* ▸, *Ztlumit / Zrušit ztlumení*. V režimu Klonovat
  navíc *Nastavit jako zdroj klonu* a *Převrátit* (zrcadlově; zdroj převrátit
  nelze — WE: „At least one display must be the source display and cannot be
  flipped“) **[moje volba podle WE — „celé“]**.
- **Dnešní zaškrtávátka zmizí.** Monitor bez tapety je monitor, kterému někdo
  dal *Odebrat* (nebo ji zatím nedostal). V režimu Klonovat jde klon na
  všechny připojené monitory kromě těch, kterým někdo dal *Odebrat* — to
  vyřadí jen ten monitor z klonu, *Změnit tapetu* ho vrátí. Odebrat na zdroji
  klonu nebo *Vypnout* ve stavovém řádku vypne klon celý. Monitory, které
  dnes nejsou zaškrtnuté (`remembered.screens`), se při prvním spuštění nové
  verze přenesou jako vyřazené z klonu, takže televize nebo projektor, který
  někdo nechal bez tapety, ji nedostane **[moje volba]**. Vyřazení z klonu se
  pamatuje.
- Výchozí rozložení je Tapeta na každý monitor i pro dosavadní instalace;
  protože při startu nic nehraje, nikoho to nepřekvapí uprostřed práce
  **[moje volba]**.
- *Kopírovat na* přenese tapetu a její vlastnosti, ne playlist
  **[moje volba]**. *Prohodit s* prohodí tapetu, playlist i vlastnosti.
- Monitory číslované zleva doprava, shora dolů podle polohy **[moje volba]**;
  popis „Monitor 1 · <výrobce a model z Qt, jinak název konektoru>“.

### Knihovna

- Nad panelem podrobností „Monitor N“ a tlačítko *Vybrat monitor*, které
  otevře nákres monitorů; klik na monitor ho vybere a nákres zavře. Jen při
  víc monitorech v režimu Tapeta na každý monitor.
- **Dvojklik / Enter na tapetu ji pustí na vybraný monitor.**
- Výběr monitoru se **sám otevře, když se otevře okno** a monitorů je víc;
  zaškrtávátko *Zobrazit při otevření* ho vypne (WE *Show on start*, výchozí
  zapnuto). Poslední vybraný monitor se pamatuje.
- **Pruh playlistu** dole ukazuje playlist vybraného monitoru; Načíst,
  Uložit, Nastavit a Přidat tapetu pracují s ním. Spuštění playlistu na
  monitoru ho dá na pruh toho monitoru.
- **Panel vlastností** patří vybranému monitoru.
- Zbarvení okna podle tapety, zvýraznění hrající karty a řádek ve stavovém
  řádku („co hraje“, *Vypnout*) ukazují vybraný monitor **[moje volba]**.
- Tlačítko *Playlisty* u filtrů spouští/zastavuje na vybraném monitoru.

### Zvuk

- Hraje každý monitor, který není ztlumený — video, hudba scény, web.
  Hudba scény se hraje jednou **za přehrávač** (v klonu tedy jednou, jako
  dnes; při dvou scénách na dvou monitorech dvakrát, každá svou).
- Ztlumení po monitoru se pamatuje. Hlavní ztlumení ve stavovém řádku =
  *Ztlumit vše*, a funguje jako pauza: dvě nezávislé věci. *Zrušit ztlumení
  všeho* nevrátí zvuk monitoru, který byl ztlumený zvlášť. Dnešní uložené
  `picker.muted` se stane ztlumením všeho **[moje volba podle pauzy]**.
- V klonu je zvuk jeden: reproduktor v nákresu a *Ztlumit* jsou jen u zdroje
  klonu a ztlumí celý klon (WE: ztlumení skupiny patří skupině)
  **[moje volba]**. Hlasitost zůstává jedna globální (dnešní posuvník)
  **[moje volba; WE má hlasitost ve vlastnostech tapety, my ji nemáme]**.

### Tray

S víc monitory v režimu Tapeta na každý monitor:

```
WallFlux
├─ Vybrat tapetu…
├─ ─────────
├─ Monitor 1 · eDP-1 — Dark Core           ▸ ├─ Pozastavit / Pokračovat
│                                             ├─ Předchozí tapeta
│                                             ├─ Další tapeta
│                                             ├─ Ztlumit / Zrušit ztlumení
│                                             ├─ Nedávné tapety ▸
│                                             ├─ Playlisty ▸
│                                             └─ Odebrat tapetu
├─ Monitor 2 · ENC CS2420 — DOOM           ▸  (totéž)
├─ ─────────
├─ Pozastavit vše / Pokračovat vše
├─ Ztlumit vše / Zrušit ztlumení
├─ Profily ▸
├─ ─────────
└─ Ukončit
```

- *Předchozí / Další* na monitoru: přes jeho běžící playlist, jinak přes
  knihovnu v pořadí mřížky od tapety toho monitoru (dnešní `step()`).
- *Nedávné tapety*: jeden společný seznam (dnešní `Wallpapers.recent`), v
  nabídce každého monitoru; vybraná jde na ten monitor. Tapeta, která na tom
  monitoru právě hraje, je zašedlá.
- *Playlisty*: uložené playlisty; klik spustí na tom monitoru, klik na
  běžící ho zastaví; běžící označený slovy, ne zaškrtnutím (dnešní pravidlo).
- **Pauza: dvě nezávislé věci.** Pauza monitoru a pauza všeho. *Pokračovat
  vše* ruší jen pauzu všeho; monitor pozastavený zvlášť stojí, dokud ho
  uživatel nepustí zvlášť (uživatelovo rozhodnutí).
- Bez položek se zaškrtnutím (dnešní pravidlo traye).
- Popisek ikony (tooltip): s víc monitory jména tapet všech monitorů po
  řádcích **[moje volba]**.

### Zakrytí a ostatní důvody k pauze

- Nastavení › Výkon, „okno přes tapetu“: k dnešním *Hrát dál / Ztlumit /
  Pozastavit / Zastavit* přibude rozlišení **jen zakrytý monitor** a
  **všechny** pro Pozastavit i Zastavit (WE: *Pause per monitor* / *Pause
  all*, *Stop* / *Stop all*). Ztlumit při zakrytí ztlumí jen zakrytý monitor
  **[moje volba]**. „Všechny“ = stačí, aby byl zakrytý jeden monitor, a stojí
  všechny. Uložené dnešní „pause“/„stop“ se čtou jako „jen zakrytý“, což se v
  klonu chová přesně jako dnes (klon je zakrytý, až když jsou zakryté všechny
  jeho obrazovky).
- Ostatní důvody (jiný program má klávesnici, jiný program hraje zvuk,
  obrazovky zhasnuté nebo zamčené, baterie) zůstávají pro všechny monitory
  **[moje volba; po monitoru se dohodlo jen zakrytí]**.
- `ScreenCover` už zná zakryté výstupy po jménech (`m_coveredScreens`); vystaví
  je, každý přehrávač si z nich vezme ty svoje.

### Přepínání rozložení

- Klon → Tapeta na každý monitor: každý monitor začne s tapetou, která se
  klonovala (a jejími vlastnostmi); playlist zůstane na prvním monitoru.
- Tapeta na každý monitor → Klon: klonuje se tapeta (a playlist) vybraného
  monitoru.

### Odpojený a připojený monitor

- Přehrávač monitoru se pozná podle jména konektoru. Odpojený monitor
  ztratí svou plochu, ale přehrávač si do konce běhu programu drží tapetu,
  playlist (hodiny playlistu stojí), ztlumení a pauzu. Po připojení pokračuje.
- Nově připojený monitor, který program v tomto běhu neviděl, začne bez
  tapety.

### Profily

- Profil = rozložení a pro každý monitor (podle jména konektoru): tapeta,
  běžící playlist, ztlumení a vlastnosti tapety **[obsah moje volba]**.
- Uložit / Načíst v kartě Monitory (dialogy jako u playlistů: přejmenovat,
  smazat s potvrzením); *Profily* ▸ v trayi načte. Načtení profilu spustí jeho
  tapety a playlisty — je to pokyn uživatele, ne start programu.
- Monitor z profilu, který teď není připojený: jeho část se drží v
  přehrávači a použije se, až se připojí. Připojený monitor, o kterém profil
  nic neříká, zůstane, jak je **[moje volba]**.
- Soubor `profiles.json` vedle `playlists.json`
  (`QStandardPaths::AppConfigLocation`).

### Co se pamatuje mezi spuštěními

Pamatuje se: rozložení, ztlumení po monitoru, poslední vybraný monitor,
*Zobrazit výběr při otevření*, vlastnosti tapet po monitoru, profily,
playlisty. **Nepamatuje se**, co na kterém monitoru hrálo, ani pauza. Pamatuje se
ještě, které monitory jsou vyřazené z klonu.

## Návrh stavby (moje volby)

- **`Display`** (nový QML objekt, ne okno), jeden na přehrávač. Drží: id
  tapety (místo `Wallpapers.current`), běžící playlist, pauzu monitoru,
  ztlumení, přechod (`arriving`/`fading`/`waiting` + `rescue` z `Main.qml`),
  hudbu scény (`SceneSound` Repeater) a důvody k pauze, které se ho týkají.
  Jeho obrazovky: v režimu po monitoru jedna, v klonu všechny; první kreslí,
  ostatní zrcadlí (`Mirror`, `firstPicture`), jak dnes.
- **`Main.qml`** drží přehrávače podle jména konektoru — vytvořené, když se
  jméno poprvé objeví, a nechané do konce běhu — a pauzu všeho. `step()`,
  `changeTo()`, `swapNow()` se přesunou do `Display`.
- **`Playlists` se rozdělí**: singleton dál drží uložené playlisty (`all`,
  `save`, `forget`, `rename`, `named`, `staysFor`, `asClock`) a soubor. Běh
  (`playing`, `start`, `stop`, `next`, `previous`, `hold`, časovač, míchání,
  `changesOnVideoEnd`, `changesWhilePaused`, `transition`, `transitionMs`,
  signál `plays`) se stane typem, kterého má každý `Display` jeden.
  Přejmenování běžícího playlistu se musí dostat do všech běhů; smazání
  běžícího ho zastaví tam, kde běží.
- **`WallpaperCore`**: `current` přestane být „ta tapeta“. Zůstane hledání
  tapety podle id, `recent` (společný) a zápis do `recent` při puštění.
  Ochrana „select() mlčí, když se nic nezměnilo“ se přenese na místo, kde
  `Display` mění tapetu (s testem).
- **`WallpaperSurface`** dostane od svého `Display` tapetu (zdroj, id) místo
  čtení `Wallpapers.current` a `Playlists`.
- **`Picker`**: `selectedDisplay` (jméno konektoru, pamatuje se); vše, co
  dnes čte `Wallpapers.current` nebo `Playlists.playing`, čte vybraný
  `Display`. `propertiesOf` se klíčuje monitorem a id; dnešní uložené hodnoty
  (klíč jen id) slouží jako výchozí pro všechny monitory, dokud monitor nemá
  své.
- **`DisplaysView`**: rozložení, nákres s náhledy, reproduktorem, výběrem a
  kontextovou nabídkou, profily. Nákres jako komponenta znovu použitá ve
  výběru monitoru v knihovně.
- **Profily**: malý C++ singleton po vzoru `Playlists` (soubor, `all`,
  `save`, `forget`, `rename`, `named`), aby čtení a zápis šly otestovat.
- **Tray** (`Tray`, C++): umí podnabídky v podnabídkách — ověřit přes D-Bus
  (`GetLayout`), ne podle QML (past v CLAUDE.md).

## Ověření

- `ctest`, `qmllint` přes **všechny** soubory v `qml/`, `cargo test` jen kdyby
  se sáhlo do `crates/`.
- Testy: běh playlistu (přesunutý `tst_playlists`), profily (soubor), mlčení
  při stejné tapetě.
- Dva monitory v `kwin_wayland --virtual --output-count 2` pod
  `dbus-run-session`: různé tapety na každém (snímek obrazovky), klon stejný
  jako dnes (oba snímky totožné na pixel), zakrytí jen jednoho monitoru
  zastaví jen ten, zvuk dvou tapet naráz (`parec` na monitoru výstupu),
  tray přes D-Bus.
- Notebook s monitorem: dvě scény naráz změřit (CPU, GPU, snímky), `-j2`,
  nikdy `pkill -x wallflux`.
- Karty *Ovládání* v Nastavení a CLAUDE.md upravit, než se vydá.

## Co zbývá staviteli

Jména typů a souborů, rozvržení prvků v kartě Monitory a ve výběru monitoru
(podle současného vzhledu programu), pořadí prací, jak přesně se přehrávače
předávají povrchům, ikony.
