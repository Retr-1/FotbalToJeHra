# Fotbal To Je Hra

3D taktický futbal v Ruste (Macroquad), 5 proti 5. Modrý tím používa Mixamo X Bot, oranžový Y Bot. Modely sú súčasťou `assets/`.

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

- Na začiatku každého kola je **15 sekúnd** na rozostavenie. Ľavým tlačidlom potiahni svojich hráčov na ľubovoľné voľné miesto svojej polovice. Prekrývanie hráčov nie je povolené.
- Po odpočítaní začne hra automaticky. Podrž ľavé tlačidlo na hráčovi, namier žltú šípku a pusti. Hráč ide k zvolenému miestu. Príkaz môžeš kedykoľvek zmeniť a postupne zadávať všetkým piatim hráčom; pohybujú sa súčasne.
- Hráč pri kontakte kope do lopty podľa smeru kontaktu a pohybu. Lopta sa spomaľuje a odráža od hraníc ihriska mimo bránok.
- Celá lopta za bránkovou čiarou znamená bod pre útočiaci tím. Lopta aj formácie sa vrátia na začiatok a znova plynie 15 sekúnd. Skóre zostáva.
- **Escape** otvorí menu a ukončí sieťové spojenie. Nová hra vynuluje skóre.

## Multiplayer cez TCP

Hostiteľ ovláda modrý tím, pripojený hráč oranžový. Fyziku, rozostavenie, čas a skóre počíta hostiteľ; klient posiela len príkazy pre svoj tím. Hra čaká na pripojenie druhého hráča. Po odpojení sa zastaví; nový zápas vytvor cez menu.

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

Pozri [assets/README.md](assets/README.md) pre zdroje. GLB modely sa načítajú pri štarte a zobrazujú sa v základnej póze, s tímovým sfarbením a otáčaním podľa smeru pohybu. Skeletálne animácie z GLB sa zatiaľ neprehrávajú. Ak súbor chýba alebo je poškodený, hra použije náhradného robota a vypíše príčinu do konzoly.

Vlastné exporty nahraď v `assets/x_bot.glb` a `assets/y_bot.glb`. FBX z Mixamo najprv importuj do Blenderu a exportuj ako GLB s geometriou postavy, bez Draco kompresie, os Y nahor. Importér zachová transformácie uzlov a automaticky upraví výšku na 1,8 hernej jednotky. Textúry a materiály nahrádza tímovým sfarbením.

## Overenie

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --smoke-test
```

Testy overujú skórovanie, reset kola, obmedzenia rozostavenia, odmietnutie príkazov súpera a starého kola, pozastavenie lopty počas rozostavenia, TCP prenos a import oboch pribalených modelov. Grafický smoke test otvorí hru, uloží `smoke-test.png` a skončí; vyžaduje grafické prostredie.
