# Postavy Mixamo

## Aktívne súbory dodané používateľom

- `models/X Bot.fbx` → `generated/x_bot.glb`
- `models/Y Bot.fbx` → `generated/y_bot.glb`
- `animations/Running.fbx` → `generated/running.glb`
- `animations/Idle.fbx` → `generated/idle.glb`
- `models/ball.glb` sa používa priamo, s pôvodnými farbami materiálov a mierkou prispôsobenou fyzikálnemu polomeru lopty.

Konverzia: FBX2glTF 0.13.1 z https://github.com/godotengine/FBX2glTF/releases/tag/v0.13.1, bez Draco kompresie. Opakovanie: `tools/convert-assets.ps1`. Zdrojové FBX zostávajú nezmenené. Konvertor pri týchto FBX upozorňuje na odlišné pravidlá dedenia transformácií; výsledné modely aj beh sú overené importnými testami a grafickými náhľadmi.

Hra používa vlastnú normalizáciu mierky a tímové sfarbenie. Animácie sa prenášajú na oba modely cez samostatné referenčné kostry. Súbory `.import`, `.anim` a `.res` pochádzajú z Godotu a táto Rust hra ich nepotrebuje. Ostatné modely sú ponechané na prípadné neskoršie použitie.

## Staršie GLB súbory (hra ich už nepoužíva)

- `x_bot.glb`: X Bot od Adobe Mixamo, GLB kópia z https://github.com/aabeling/something-with-threejs/blob/master/Xbot.glb
- `y_bot.glb`: Y Bot od Adobe Mixamo, GLB kópia z https://github.com/aabeling/something-with-threejs/blob/master/Ybot.glb
- Stiahnuté 1. novembra 2026. Zdrojový projekt uvádza pôvod šablóny https://github.com/mterczynski/typescript-webpack-threejs-template.

Postavy sú obsah Adobe Mixamo; licencia zdrojového kódu hry nemení podmienky ich používania. Informácie Adobe: https://helpx.adobe.com/creative-cloud/faq/mixamo-faq.html. Originálne exporty možno získať po prihlásení na https://www.mixamo.com/.

Staršie súbory `x_bot.glb` a `y_bot.glb` sú ponechané; na aktuálne modely a animácie hra používa priečinok `generated/`.
