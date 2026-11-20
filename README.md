# rust

Две игры на Rust → WebAssembly:

- **Forest Bloom 3D** (`game3d/`) — полноценный 3D FPS: лес из glTF-ассетов, меч, волны врагов, меню, настройки, пауза. Готовый автономный HTML лежит в GitHub Release (`ForestBloom3D.html`).
- **Forest Bloom** (`game/`) — ранняя 2D-версия на Canvas2D.

Разбор архива ассетов: `inventory.json`, `deep_inventory.json.gz`, `inspection/`.
Workflow для загрузки и разбора архива: `.github/workflows/inspect-drive-assets.yml`.

Быстрый старт 3D-версии:

```bash
cd game3d
./build.sh
python3 serve.py   # http://localhost:8081
```
