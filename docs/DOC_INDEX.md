# Documentation Index

Audit of every top-level markdown doc in `scenic-motion-engine-mac`. Generated 2026-11-02.

## Doc summaries

- **`README.md`** — Native macOS port of the audio-reactive looping marquee; four-band transient detection drives per-row scrolling, camera moves, and spotlight repetition on wgpu/Metal with native audio I/O and MP4 export.
- **`MOTION.md`** — Motion architecture for the Rust port: `engine::evaluate(EngineInputs) -> FrameState` as a pure, stateless function of time, with the full `EventSource -> Tickers -> PhaseSequences -> Paths -> Bindings -> Renderer` chain.
- **`CHANGELOG.md`** — Keep-a-Changelog / SemVer log tracking project releases; currently records the `0.1.0` initial native macOS port landing on 2026-11-02.
- **`CONTRIBUTING.md`** — Local setup, required pre-push checks (`cargo fmt --check`, `clippy -D warnings`, `test`, `build --release`), and the PR flow for the contract-first swarm pattern around `src/types.rs`.
- **`SECURITY.md`** — Scope and reporting policy for the native-only macOS app that reads local assets and spawns `ffmpeg`; private disclosure via email with a 7-day best-effort response window.
- **`THIRDPARTY.md`** — License/version/purpose table for every direct crate dependency (`[dependencies]` in `Cargo.toml`), from `eframe` and `wgpu` through `cosmic-text`, `cpal`, `symphonia`, and the `serde` family.
- **`docs/USER_GUIDE.md`** — First-run macOS walkthrough covering prerequisites, dropping fonts/audio/analysis assets, launching, GUI panel sections, the record flow, and troubleshooting.
- **`docs/MOTION_RECIPES.md`** — Cookbook of worked `tickers + bindings` JSON recipes for common musical patterns (half-time snare, 4th-kick zoom, bar dolly, strong-hats jitter, see-saw, scale breathe, kick-fill flicker).
- **`completions/README.md`** — One-paragraph instructions for installing the `render` binary's bash/zsh/fish shell completions and the `render.1` manpage.
- **`website/README.md`** — How to serve the static project site locally with `python3 -m http.server` and host it on GitHub Pages from the `website/` folder.
- **`.github/REPO_SETUP.md`** — One-time GitHub configuration steps: enable Pages (GitHub Actions source), set discoverability topics, and switch on Discussions.

> Not present at scan time: `docs/ARCHITECTURE.md`, `docs/PATH_REFERENCE.md`, `docs/PERFORMANCE.md`, `examples/configs/README.md`.

---

## Internal-link audit

Only `[text](path)` relative links are listed. External `https://` URLs and in-page `#anchors` are noted as `external` or omitted.

| Doc | Link | Status |
| --- | --- | --- |
| `README.md` | `docs/screenshots/app.png` | OK |
| `README.md` | `./LICENSE` (license badge) | OK |
| `README.md` | `./MOTION.md` (Architecture section) | OK |
| `README.md` | `./src/types.rs` | OK |
| `README.md` | `./MOTION.md` (Motion extension section) | OK |
| `README.md` | `./rustfmt.toml` | OK |
| `README.md` | `./clippy.toml` | OK |
| `README.md` | `https://github.com/...` (badges, upstream, acks) | external |
| `MOTION.md` | _(no relative markdown links)_ | — |
| `CHANGELOG.md` | `./README.md` | OK |
| `CHANGELOG.md` | `./README.md#status` | OK |
| `CHANGELOG.md` | `./CONTRIBUTING.md` (×2) | OK |
| `CHANGELOG.md` | `./MOTION.md` | OK |
| `CHANGELOG.md` | `./SECURITY.md` | OK |
| `CHANGELOG.md` | `https://keepachangelog.com/...`, `https://semver.org/...`, `https://github.com/...` | external |
| `CONTRIBUTING.md` | `./README.md` | OK |
| `CONTRIBUTING.md` | `./README.md#install` | OK |
| `CONTRIBUTING.md` | `./clippy.toml` | OK |
| `CONTRIBUTING.md` | `./rustfmt.toml` | OK |
| `CONTRIBUTING.md` | `./.github/CODEOWNERS` | OK |
| `CONTRIBUTING.md` | `./CHANGELOG.md` | OK |
| `CONTRIBUTING.md` | `./MOTION.md` | OK |
| `CONTRIBUTING.md` | `https://rustup.rs` | external |
| `SECURITY.md` | `./README.md` | OK |
| `THIRDPARTY.md` | `https://github.com/...`, `https://wgpu.rs`, `https://serde.rs` | external |
| `docs/USER_GUIDE.md` | _(no relative markdown links)_ | — |
| `docs/MOTION_RECIPES.md` | _(no relative markdown links)_ | — |
| `completions/README.md` | _(no relative markdown links)_ | — |
| `website/README.md` | _(no relative markdown links)_ | — |
| `.github/REPO_SETUP.md` | _(no relative markdown links)_ | — |

Result: 0 broken relative links across 11 docs scanned. All 20 relative `./path` and `path/to/file` targets resolve.

---

## Suggested cross-links not yet present

The following links would improve navigation but are not currently in the docs. These are suggestions only — no file was edited as part of this audit.

- **`CONTRIBUTING.md` should link to `.github/PULL_REQUEST_TEMPLATE.md`.** It currently references the template as `.github/pull_request_template.md` in prose (lowercase, no link). GitHub resolves the file by case-insensitive lookup, so the on-disk uppercase file (`.github/PULL_REQUEST_TEMPLATE.md`) is what gets served. Make it an actual markdown link so readers can click through.
- **`README.md` should link to `docs/USER_GUIDE.md`.** The `Install` / `Run` sections duplicate first-run steps that `docs/USER_GUIDE.md` covers more thoroughly; a "First-run walkthrough" link under `Run` would surface the guide.
- **`README.md` should link to `docs/MOTION_RECIPES.md`.** The `Motion extension` section only links to `MOTION.md`; adding "See `docs/MOTION_RECIPES.md` for drop-in ticker + binding examples" would route readers from the extension contract to concrete examples.
- **`README.md` should link to `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, and `THIRDPARTY.md`.** None of the four sibling top-level docs are linked from the README. Standard practice is a "Project meta" or footer section that lists them.
- **`CONTRIBUTING.md` should link to `SECURITY.md`.** Security-sensitive reports are explicitly routed away from public issues; `CONTRIBUTING.md` currently never mentions this.
- **`MOTION.md` should link to `docs/MOTION_RECIPES.md`.** The recipes are the natural companion to the path-extension contract.
- **`docs/USER_GUIDE.md` should link back to `README.md` and `MOTION.md`.** The guide is a standalone first-run doc with no outbound links; a "See also" footer would complete the loop.
- **`docs/MOTION_RECIPES.md` should link to `MOTION.md`.** The recipes reference path names from `src/motion/paths.rs` and compose modes from `MOTION.md`, but neither is linked.
- **`CHANGELOG.md` should link to `THIRDPARTY.md`** when dependency changes land under a `### Changed` section.
- **`website/README.md` should link to the top-level `README.md`** so visitors browsing the site repo understand it is a sub-component of the main project.
- **`.github/REPO_SETUP.md` should link to `.github/workflows/pages.yml`** (mentioned in prose but not linked) and to the top-level `README.md`.
- **`completions/README.md` should link to `docs/man/render.1`** rather than referring to it only by relative path in prose.
