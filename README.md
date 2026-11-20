# rust

Игра **Forest Bloom** — логика на Rust → WebAssembly, рендер в браузере.

- Игра: `game/` (см. `game/README.md`)
- Анализ архива ассетов: `inventory.json`, `deep_inventory.json.gz`, `inspection/`
- Workflow разбора архива через GitHub Actions: `.github/workflows/inspect-drive-assets.yml`

Быстрый старт:

```bash
cd game
./build.sh
python3 serve.py   # http://localhost:8080
```
