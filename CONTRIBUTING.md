# Contributing to scenic-motion-engine-mac

Thanks for your interest. This document covers local setup, the required
pre-push checks, and the PR flow. For project context, feature matrix, and
architecture, start at [README.md](./README.md).

---

## Setup

Prerequisites — same as the [README install section](./README.md#install):

- macOS 13+
- Rust 1.75+ via [`rustup`](https://rustup.rs)
- `ffmpeg` on `PATH` (for MP4 export)

Clone and fetch:

```bash
git clone https://github.com/danielesiegel/scenic-motion-engine-mac.git
cd scenic-motion-engine-mac
rustup show        # confirm 1.75+ stable toolchain is active
cargo fetch
```

Drop assets before first run:

- A font (`.otf` / `.ttf`, ITC Avant Garde Gothic LT Demi preferred) into `assets/fonts/`
- A WAV into `assets/audio/`
- The matching analysis JSON into `analysis/` (produced by `scripts/analyze.py` in the web repo)

Then:

```bash
make run
```

See `make help` for the full target list (`run`, `render`, `check`, `bundle`,
`analyze`).

---

## Local checks

Run all four before pushing — CI will run the identical set via
`.github/workflows/ci.yml` and reject anything that fails:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

Clippy is pinned via [`clippy.toml`](./clippy.toml); formatting via
[`rustfmt.toml`](./rustfmt.toml). Do not relax either without a PR explaining
why.

Benches live under `benches/` (criterion). Run with `cargo bench` when
touching the engine hot path.

---

## Opening PRs

- Branch off `main`, keep PRs small and focused.
- Fill in the PR template (`.github/pull_request_template.md`) — Context,
  Changes, Test Plan.
- Reviewers are routed via [CODEOWNERS](./.github/CODEOWNERS); expect at least
  one owner review on every file touched.
- CI must be green before merge.
- Update [CHANGELOG.md](./CHANGELOG.md) under `## [Unreleased]` for any
  user-visible behavior change.

---

## Motion extensions

Adding a new path function, easing, or ticker binding? Read **[MOTION.md](./MOTION.md)** first — it covers the path-function contract, continuity model, compose modes, per-glyph outputs, and the registered-paths table.

New paths go in `src/motion/paths.rs` and get a one-line entry in MOTION.md's
registered-paths table.

---

## Contract-first swarm pattern

This project was prototyped with a small agent swarm — one agent per module
(engine, text, audio, motion, render, gui, record), coordinated by an
orchestrator. The invariant that made parallel work tractable:

> **`src/types.rs` is the immutable contract.** Every module builds against
> these types. Module agents may add helper types inside their own module but
> must not change `types.rs` without the orchestrator.

The same rule holds for human contributors. If you need to change
`src/types.rs`:

1. Open an issue or discussion first describing the shape change.
2. Expect the PR to touch every downstream module that consumes the type.
3. Keep the diff minimal — add new fields with `#[serde(default)]` rather than
   rewriting existing ones.

This keeps module boundaries narrow and makes it possible to review engine,
render, and audio changes independently.

---

## Code of conduct

Be kind. Prefer evidence over opinion. If a reviewer asks for a benchmark,
include one.
