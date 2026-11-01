# Fotbal To Je Hra

**Android APK:** spusti Docker Desktop s Linux kontajnermi a potom `powershell -ExecutionPolicy Bypass -File .\build-android.ps1`, rovnako ako v RicochetRobots. Android SDK, NDK, Java aj Rust sa pripravia v kontajneri. Výsledok: `target/android-artifacts/release/apk/game.apk`. Podrobnosti sú v [android/README.md](android/README.md).

3D taktický futbal v Ruste (Macroquad), 4 proti 4. Červený tím používa Mixamo X Bot, modrý Y Bot. Bránky majú šírku 8,4 hernej jednotky, takže traja obrancovia nepokryjú celý priestor pre loptu. Hra používa tvoje FBX modely a animácie prevedené do `assets/generated/`.

## Spustenie

Vyžaduje Rust a Cargo. Prvý build stiahne závislosti.

```powershell
cargo run --release
```

Menu ponúka počítačového súpera, vytvorenie multiplayera a pripojenie podľa IP adresy. Rýchle spustenie proti počítaču:

```powershell
cargo run --release -- --ai
```

Spúšťaj z koreňového priečinka projektu, aby hra našla `assets/`.

## Ovládanie a pravidlá

- Na začiatku každého kola je **5 sekúnd** na rozostavenie. Ľavým tlačidlom potiahni svojich hráčov na ľubovoľné voľné miesto svojej polovice. Prekrývanie hráčov nie je povolené.
- Po odpočítaní začne hra automaticky. Podrž ľavé tlačidlo na hráčovi, namier žltú šípku a pusti. Hráč ide k zvolenému miestu. Príkaz môžeš kedykoľvek zmeniť a postupne zadávať všetkým štyrom hráčom; pohybujú sa súčasne.
- Hráč pri kontakte kope do lopty podľa smeru kontaktu a pohybu. Lopta sa spomaľuje a odráža od hraníc ihriska mimo bránok.
- Celá lopta za bránkovou čiarou znamená bod pre útočiaci tím. Na jednu sekundu sa zobrazí `GOAL!` vo farbe skórujúceho tímu; postavy zostanú stáť a lopta pokračuje dovnútra bránky, kde sa odráža od siete a spomaľuje. Až potom sa lopta aj formácie vrátia na začiatok a spustí sa päťsekundové rozostavenie. Skóre zostáva.
- Lopta pod tlakom: stlačenie medzi hráčmi alebo medzi hráčom a mantinelom postupne zvyšuje pružnosť lopty a silu odrazu. Nahromadený tlak vytláča loptu cez najvoľnejšiu medzeru a zatlačí postavy späť. Po uvoľnení rýchlo klesá. Rýchlosť lopty je obmedzená na 18 jednotiek/s. Lopta sa nepresúva na iné miesto a hra sa neprerušuje. Súčasné kontakty sa sčítavajú, namiesto prepisovania posledným hráčom. Rohy sú zaoblené s polomerom 2,5 jednotky a odrážajú loptu podľa normály oblúka. Rovnaká hranica platí aj pre postavy a rozostavenie. Fyziku multiplayera vyhodnocuje hostiteľ.
- **Escape** otvorí menu a ukončí sieťové spojenie. Nová hra vynuluje skóre.

## Multiplayer cez TCP

Pri vytvorení hry sa do pripojenia súpera zobrazuje lokálna IP adresa hostiteľa a port. Po pripojení zostáva skóre a tlačidlo Menu; počas päťsekundového rozostavenia sa zobrazuje aj odpočet. Ihrisko využíva celú dostupnú plochu bez horných a dolných textových líšt; kamera zachováva pomer strán a viditeľnosť oboch bránok.

Hostiteľ ovláda červený tím (X Bot), pripojený hráč modrý (Y Bot). Fyziku, rozostavenie, čas a skóre počíta hostiteľ; klient posiela len príkazy pre svoj tím. Hra čaká na pripojenie druhého hráča. Po odpojení sa zastaví; nový zápas vytvor cez menu.

Na prvom počítači:

```powershell
cargo run --release -- --host 0.0.0.0:7777
```

Na druhom počítači (nahraď adresu IP hostiteľa):

```powershell
cargo run --release -- --join 192.168.1.10:7777
```

Na jednom počítači otvor dve inštancie; klient použije `127.0.0.1:7777`. Pre LAN musí firewall hostiteľa povoliť prichádzajúci TCP port 7777. Cez internet potrebuje hostiteľ dostupnú verejnú adresu a presmerovaný port alebo spoločnú VPN; hra neobsahuje matchmaking ani relay server. Pripojenie je určené pre známych spoluhráčov a nie je šifrované.

## Modely

Lopta používa dodaný `assets/models/ball.glb`, vrátane pôvodných svetlých a čiernych panelov. Geometria sa automaticky vycentruje a zmenší na fyzikálny polomer 0,45 jednotky. Pri chýbajúcom modeli sa použije náhradná guľa.

Pozri [assets/README.md](assets/README.md) pre zdroje. GLB modely sa načítajú pri štarte s kostrou a váhami vrcholov. Oba tímy používajú tvoje animácie `Idle.fbx` a `Running.fbx`, prevedené do GLB; na každého bota sa prenášajú podľa názvov kostí a rozdielov ich orientácie v základnej póze. Každá animácia má vlastnú referenčnú kostru, takže nevyžaduje rovnaké poradie kostí. Medzi státím a behom sa plynulo prelína podľa skutočnej rýchlosti hráča, aj pri sieťovej hre. Horizontálny posun animácie je odstránený, takže pozíciu a kolízie naďalej riadi hra. Počas rozostavenia sa prehráva státie. Ak súbor chýba alebo je poškodený, hra použije náhradného robota a vypíše príčinu do konzoly.

Po výmene FBX zdrojov spusti `powershell -ExecutionPolicy Bypass -File tools/convert-assets.ps1`. Skript používa FBX2glTF 0.13.1; vlastnú cestu možno zadať cez `-ConverterPath`. Vygenerované GLB sú pribalené, takže na bežné spustenie hry konvertor netreba. Importér zachová transformácie uzlov a automaticky upraví výšku na 1,8 hernej jednotky. Textúry a materiály nahrádza tímovým sfarbením. Súbory Godot `.import`, `.anim` a `.res` ani ostatné modely z `assets/models/` hra zatiaľ nenačítava.

## Overenie

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --smoke-test
cargo run -- --smoke-run=0.2
```

Testy overujú skórovanie, reset kola, obmedzenia rozostavenia, odmietnutie príkazov súpera a starého kola, pozastavenie lopty počas rozostavenia, TCP prenos, deformovanie oboch modelov animáciou behu, zachovanie pozície koreňa a prechod medzi státím a behom. Grafický smoke test otvorí hru, uloží `smoke-test.png` a skončí; vyžaduje grafické prostredie. `--smoke-run=0.2` uloží do `smoke-run.png` náhľad oboch tímov v čase 0,2 sekundy bežeckého klipu (bez zmeny pravidiel hry).
