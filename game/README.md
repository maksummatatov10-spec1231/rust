# Forest Bloom

Небольшая браузерная аркада, вся игровая логика которой написана на Rust и скомпилирована в WebAssembly. Внешние crates не используются — только `std` и встроенный таргет `wasm32-unknown-unknown`.

## Запуск

```bash
./build.sh       # cargo build --release --target wasm32-unknown-unknown
python3 serve.py # http://0.0.0.0:8080
```

`serve.py` отдаёт `application/wasm` для `core.wasm`.

## Управление

- `WASD` / стрелки — движение
- мышь (нажатие) / пробел / `J` — стрельба в сторону курсора
- `P` — пауза
- `Enter` или пробел — старт/рестарт

## Что где

- `src/lib.rs` — состояние игры, спавнеры, коллизии, очки, волны, event queue.
- `main.js` — Canvas2D-рендер, ввод, звук, HUD.
- `index.html` — интерфейс.
- `core.wasm` — готовый артефакт (48 КБ).
- `assets/` — отобранные из архива текстуры и звуки.

## Использованные ассеты

Из `assets.zip` (полная опись — `../inventory.json`, `../deep_inventory.json.gz`, `../inspection/`):

- `Stylized Nature MegaKit_Standard_` — `Grass`, `Flowers`, `Mushrooms`, `Leaves_NormalTree`, `PathRocks_Diffuse`
- `KayKit_FantasyWeaponsBits`, `KayKit_HalloweenBits`, `KayKit_RPGToolsBits` — палитры/текстуры
- `BattleFXFree` — спрайт листа
- `JDSherbert - Pixel UI SFX Pack` и `Sci Fi UI SFX Pack` — звуки UI

Архив содержит 1 264 737 618 байт, 31 файл верхнего уровня, 29 вложенных архивов, 6 101 запись внутри них. Полный разбор хранится в корне репозитория.
