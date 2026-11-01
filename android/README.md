# Natívna Android aplikácia

Hra sa kompiluje do APK pre **Android 8.0+ (API 26), ARM64**, s vlastným OpenGL ES vykresľovaním. Nie je to webová stránka ani WebView. Landscape orientácia sa môže otočiť na obe strany. Desktop build zostáva dostupný cez `cargo run --release`.

## Windows: Docker ako v RicochetRobots (odporúčané)

Spusti **Docker Desktop s Linux kontajnermi**. Z koreňa projektu spusti:

```powershell
powershell -ExecutionPolicy Bypass -File .\build-android.ps1
```

Výsledok: `target/android-artifacts/release/apk/game.apk`.

Na Windows nepotrebuješ inštalovať Android Studio, SDK, NDK, JDK ani ďalší Rust toolchain. Skript zostaví a uloží Docker obraz `fotbal-android:api36`, ktorý obsahuje Rust 1.95, JDK 17, SDK/API 36, Build Tools 36.0.0 a NDK 28.2.13676358. Prvý build stiahne väčšie množstvo dát a skompiluje APK nástroj; ďalšie buildy používajú cache.

Postup vychádza z projektu RicochetRobots. Táto hra má novšie závislosti, preto používa novší Rust a kompatibilný APK builder. `Cargo.lock` zostáva vo formáte v4; zdroje hostiteľa sa nemenia. Desktop a Android majú oddelené build cache.

Docker volumes `fotbal-android-registry` a `fotbal-android-build` uchovávajú cache. `fotbal-android-signing` uchováva vývojový podpisový kľúč, aby ďalšie APK mohlo aktualizovať existujúcu inštaláciu. Tieto volumes sú samostatné od RicochetRobots. Skript overí podpis APK, ARM64 knižnicu `libgame.so`, Java loader, SDK verzie a sieťové oprávnenie. Názov binárneho cieľa `game` zodpovedá poslednej časti balíka `sk.fotbaltojehra.game`.

Ak Docker nie je spustený alebo používa Windows kontajnery, skript skončí s vysvetlením. Vytvorené APK je určené na testovacie inštalovanie; produkčné podpisovanie a distribúcia cez obchod nie sú nakonfigurované.

## Windows: alternatívny build bez Dockeru

Potrebné sú Rust, Git, JDK 17 (`JAVA_HOME`/`PATH`), 7-Zip a Android SDK/NDK. V Android Studio → SDK Manager nainštaluj:

- Android SDK Platform 36
- Android SDK Build-Tools 36.0.0
- Android SDK Platform-Tools (pre `adb`)
- NDK (Side by side) **28.2.13676358**; zapni „Show Package Details“

Alternatívne cez nainštalovaný `sdkmanager`:

```powershell
sdkmanager 'platforms;android-36' 'build-tools;36.0.0' 'platform-tools' 'ndk;28.2.13676358'
```

Potom z koreňa projektu:

```powershell
# Prvýkrát nainštaluje pripnutý Rust a APK build nástroj:
powershell -ExecutionPolicy Bypass -File tools/build-android.ps1 -Setup

# Ďalšie buildy:
powershell -ExecutionPolicy Bypass -File tools/build-android.ps1
```

Skript hľadá SDK v `ANDROID_HOME`, inak v `%LOCALAPPDATA%/Android/Sdk`. Vlastné cesty: `-SdkPath C:/Android/Sdk -NdkPath C:/Android/Sdk/ndk/28.2.13676358`. APK nástroj sa inštaluje do `target/android-tools`, bez nahradenia globálneho Cargo nástroja. Nástroj a Rust **1.95.0** sú pripnuté spolu, pretože APK builder používa interné Cargo API.

Výsledok: `target/android-artifacts/release/apk/*.apk`. `-DebugBuild` vytvorí debug variant. Skript overí podpis APK. Balík je podpísaný vývojovým kľúčom nástroja a je určený na priame testovacie inštalovanie. Publikovanie do obchodu a produkčný podpis nie sú súčasťou tejto konfigurácie.

## Build cez GitHub Actions

Workflow [Android APK](../.github/workflows/android.yml) nainštaluje SDK a NDK, spustí testy, zostaví a overí APK. Po nahratí projektu na GitHub ho možno spustiť v Actions → Android APK → Run workflow. APK bude v artefakte `fotbal-android-arm64`. Workflow je pripravený v repozitári; lokálna úprava ho sama nespúšťa.

## Inštalácia na telefón

Zapni USB debugging, pripoj telefón a povoľ prístup počítaču:

```powershell
adb devices
adb install -r target/android-artifacts/release/apk/NAZOV_BALIKA.apk
```

Nahraď názov skutočným súborom vypísaným build skriptom. APK možno tiež preniesť do telefónu a otvoriť inštalátorom.

## Ovládanie

- Jedným prstom chyť hráča a potiahni ho. Počas rozostavenia sa presúva priamo; počas hry pustenie prsta zadá smer pohybu.
- Druhý prst nepreberie rozpracované ťahanie. Prerušený dotyk sa zruší bez zadania príkazu.
- Návrat do menu: tlačidlo **Menu** vpravo hore alebo Android Back. Počas rozostavenia je odpočet 5 sekúnd uprostred pod skóre, ďalej od okrajov a výrezu displeja; po jeho skončení zmizne. Tieto prvky sú prekryté nad ihriskom bez veľkých textových líšt.
- Pri hostovaní sa uprostred zobrazuje lokálna IP adresa a port do pripojenia druhého hráča. Ak sa adresa nedá zistiť, hra vyzve na kontrolu Wi-Fi. Adresa zodpovedá aktívnemu sieťovému rozhraniu; pri VPN skontroluj, či sú obaja hráči v rovnakej sieti.
- Klepnutie na IP adresu v menu otvorí systémovú klávesnicu. Prvý napísaný znak nahradí predvolenú adresu.
- Pre multiplayer pripoj telefón a druhé zariadenie do rovnakej LAN/Wi-Fi. Na klientovi zadaj IP hostiteľa a port 7777. `127.0.0.1` označuje samotný telefón. Stav hry ide cez UDP, pripojenie a príkazy cez TCP; oba protokoly fungujú aj medzi Androidom a desktopom. Firewall hostiteľa musí povoliť **TCP aj UDP port 7777**.
- Súper a lopta používajú interpoláciu s približne 100 ms rezervou. Vlastní hráči a rozostavenie reagujú okamžite lokálnou predikciou; hostiteľ potvrdzuje príkazy a postupne koriguje prípadné rozdiely pozícií. Aktualizuj obe zariadenia na túto verziu.

Modely a animácie sú vložené priamo do natívnej knižnice cez `include_bytes!`; APK nepotrebuje prístup k úložisku ani ďalšie stiahnutie assetov. Po zmene modelov spusti konverziu FBX a vytvor nový APK. Na mobile sa používa nižšie rozlíšenie vykresľovania a vypnuté MSAA pre nižšiu záťaž GPU.

## Overenie a obmedzenia

Kontrola Rust kódu bez NDK/linkovania:

```powershell
rustup target add aarch64-linux-android
cargo check --target aarch64-linux-android
```

Táto kontrola nenahrádza zostavenie a spustenie APK. Pri vývoji treba na zariadení overiť výkon animácií, klávesnicu, multitouch a návrat z pozadia. iOS projekt a podpisovanie cez Xcode táto Android konfigurácia neobsahuje.

Build vychádza z [Android podpory Macroquad](https://macroquad.rs/articles/android/) a [pripnutej kompatibilnej verzie cargo-quad-apk](https://github.com/hoklims/cargo-quad-apk/tree/bdb61325ce1aecf09ed8969690f53c86c846c118).
