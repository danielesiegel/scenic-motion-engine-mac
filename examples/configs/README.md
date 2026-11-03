# Example `Config` JSON files

Each file in this directory is a complete, round-trippable `Config` document
that `scenic-motion-engine` can load via `config::persistence::load_config`.

## Files

| File | Purpose |
| --- | --- |
| `minimal.json` | One row, one spotlight, kick-only slide, no camera motion. |
| `maximal.json` | Exact serialization of `default_config()` — every binding and ticker. |
| `glacial.json` | Slow 2x duration scale, scale pulse on mid band, flat camera, tilt baseline 0. |
| `aggressive.json` | Fast 0.5x duration, high-amplitude glyph jitter, aggressive camera zoom + tilt. |
| `minimal-no-camera.json` | Row motion only, camera baseline identity, no camera bindings. |

## How to load one

### Option A — drop into the default config path

The app reads from
`~/Library/Application Support/scenic-motion-engine/config.json`
at startup (`config::persistence::load_or_default`). Copy the example of your
choice into that path and relaunch:

```sh
cp examples/configs/aggressive.json \
   "$HOME/Library/Application Support/scenic-motion-engine/config.json"
```

Back up the existing file first if you want to keep it:

```sh
cp "$HOME/Library/Application Support/scenic-motion-engine/config.json" \
   "$HOME/Library/Application Support/scenic-motion-engine/config.json.bak"
```

Then relaunch the app and the new config will take effect.

### Option B — paste into the GUI's "load from file" picker

If the GUI exposes a "load from file" / "import config" picker, point it at
any `.json` in this directory. The underlying call is the same
`load_config(path)` used by option A.

## Verifying a config parses

Run `verify.sh` from anywhere; it:

1. Invokes `cargo run --release --bin render -- --help` to confirm the project
   builds.
2. Loops over every `.json` here and parses it with `python3`'s `json` module
   to confirm basic syntactic validity.

```sh
./examples/configs/verify.sh
```

The full Rust-side round-trip (JSON → `Config` → JSON) is already covered by
`cargo test config::persistence::tests::round_trip_default_config`.

## Serde key conventions (for reference)

- Top-level `Config` fields use snake_case except `bandThresholds`.
- `LayoutConfig`, `KeyConfig`, `CanvasConfig`, `CameraConfig`, `FeelConfig`,
  `RecordConfig` all use `rename_all = "camelCase"`.
- `TickerConfig` and `Binding` use per-field renames: `tickerId`,
  `strengthThreshold`, `stepOffset`, `respectAlternation`.
- `CameraBaseline` uses `panX`, `panY`.
- `AudioConfig` uses `analysisUrl`.
- Enum variants (`Band`, `BindingTarget`, `ComposeMode`, `TextCase`, etc.) are
  lowercase.
- `EasingName` variants are camelCase (`easeOutCubic`).
- `RowVelocityShape` variants are kebab-case (`u`, `inverted-u`, `flat`,
  `linear`).
