# Motion Recipes

A cookbook of worked `tickers + bindings` configurations for common musical
patterns. Each recipe includes a short motivation, a drop-in JSON snippet
(ticker + binding), and the expected visual result.

Path names reference `src/motion/paths.rs`. Band thresholds reference
`bandThresholds` in `src/config/defaults.ts`.

---

## 1. Half-time snare only

**Motivation.** Classic trap / half-time feel: ignore the kick entirely and
let the snare on beats 2 and 4 drive all glyph motion. Great when the beat is
sparse and you want the backbeat to feel like the only pulse.

```json
{
  "tickers": [
    { "id": "halftime-snare", "band": "snare", "modulo": 1, "strengthThreshold": 0.5 }
  ],
  "bindings": [
    {
      "tickerId": "halftime-snare",
      "target": "rows",
      "rows": "all",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "slideStep", "params": { "units": 1 }, "duration": 0.35, "easing": "easeOutCubic" }
        ]
      }
    }
  ]
}
```

**Expected visual.** All rows slide one unit on every snare hit; kick-dense bars feel visually "empty" between the 2 and 4 slides.

---

## 2. Every 4th kick zoom-in (current default)

**Motivation.** Mirrors the existing `kick-camera-zoompan` binding in
`defaults.ts`: every fourth kick triggers a focused zoom-in, zoom-out, pan-
right, pan-left cycle. Lands cleanly on 4/4 bar lines when the kick pattern is
on-the-floor.

```json
{
  "tickers": [
    { "id": "kick-camera-zoompan", "band": "kick", "modulo": 4, "step": 4, "stepOffset": 3, "strengthThreshold": 0.25 }
  ],
  "bindings": [
    {
      "tickerId": "kick-camera-zoompan",
      "target": "camera",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "cameraZoomDeltaFocused", "params": { "deltaScale":  0.10, "focusFrac": 0.6 }, "duration": 0.45, "easing": "easeOutCubic" },
          { "path": "cameraZoomDeltaFocused", "params": { "deltaScale": -0.10, "focusFrac": 0.6 }, "duration": 0.45, "easing": "easeOutCubic" },
          { "path": "cameraPanDelta",         "params": { "deltaPx":     80            },         "duration": 0.45, "easing": "easeOutCubic" },
          { "path": "cameraPanDelta",         "params": { "deltaPx":    -80            },         "duration": 0.45, "easing": "easeOutCubic" }
        ]
      }
    }
  ]
}
```

**Expected visual.** Camera pushes into a randomized sub-region on the downbeat, releases, pans right then left — one full cycle per bar.

---

## 3. Camera dolly on bar line

**Motivation.** Continuous horizontal dolly motion that reverses direction on
every downbeat (every 4th kick). The two-step sequence alternates on each
trigger so the camera smoothly see-saws across the frame bar-by-bar.

```json
{
  "tickers": [
    { "id": "bar-dolly", "band": "kick", "modulo": 4, "step": 4, "stepOffset": 3, "strengthThreshold": 0.25 }
  ],
  "bindings": [
    {
      "tickerId": "bar-dolly",
      "target": "camera",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "cameraPanDelta", "params": { "deltaPx":  120 }, "duration": 1.8, "easing": "linear" },
          { "path": "cameraPanDelta", "params": { "deltaPx": -120 }, "duration": 1.8, "easing": "linear" }
        ]
      }
    }
  ]
}
```

**Expected visual.** A slow, bar-synced horizontal dolly — right for a bar, left for a bar, continuously.

---

## 4. Jitter only on strong hats (dubstep hats)

**Motivation.** Dense hat patterns can flood the frame if every tick triggers
shimmer. Raising `strengthThreshold` to 0.8 skips the ghost hats and only
reacts to the heavily-accented ones — the classic dubstep hat pattern.

```json
{
  "tickers": [
    { "id": "strong-hats-jitter", "band": "hats", "modulo": 1, "strengthThreshold": 0.8 }
  ],
  "bindings": [
    {
      "tickerId": "strong-hats-jitter",
      "target": "rows",
      "rows": "all",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "glyphJitter", "params": { "amplitude": 3 }, "duration": 0.12, "easing": "linear" }
        ]
      }
    }
  ]
}
```

**Expected visual.** Frame is still through ghost hats, then every accented hat punches a short per-glyph shimmer.

---

## 5. Chaotic see-saw

**Motivation.** Odd and even rows scroll in opposite directions at accelerating
speeds, driven by two different bands. The mismatch in bands + opposing
`pxPerSec` signs produces a tearing, destabilizing feel useful for breakdowns.

```json
{
  "tickers": [
    { "id": "seesaw-even", "band": "kick",  "modulo": 1, "strengthThreshold": 0.25 },
    { "id": "seesaw-odd",  "band": "snare", "modulo": 1, "strengthThreshold": 0.25 }
  ],
  "bindings": [
    {
      "tickerId": "seesaw-even",
      "target": "rows",
      "rows": "even",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "driftSpeed", "params": { "pxPerSec":  20 }, "duration": 0.5, "easing": "easeInCubic" },
          { "path": "driftSpeed", "params": { "pxPerSec":  60 }, "duration": 0.5, "easing": "easeInCubic" }
        ]
      }
    },
    {
      "tickerId": "seesaw-odd",
      "target": "rows",
      "rows": "odd",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "driftSpeed", "params": { "pxPerSec": -20 }, "duration": 0.5, "easing": "easeInCubic" },
          { "path": "driftSpeed", "params": { "pxPerSec": -60 }, "duration": 0.5, "easing": "easeInCubic" }
        ]
      }
    }
  ]
}
```

**Expected visual.** Even rows accelerate rightward on kicks, odd rows accelerate leftward on snares — a shearing, see-sawing texture.

---

## 6. Breathing scale pulse

**Motivation.** Subtle, musical "breathing" on every mid-band event (melodic
content, pads, vocals). A modest magnitude keeps the pulse legible without
hiding the text.

```json
{
  "tickers": [
    { "id": "mid-breathe", "band": "mid", "modulo": 1, "strengthThreshold": 0.3 }
  ],
  "bindings": [
    {
      "tickerId": "mid-breathe",
      "target": "rows",
      "rows": "all",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "scalePulse", "params": { "magnitude": 0.04 }, "duration": 0.6, "easing": "easeOutCubic" }
        ]
      }
    }
  ]
}
```

**Expected visual.** All glyphs gently inflate and deflate on each mid event — reads as the text "breathing" with the melody.

---

## 7. Flicker on kick fills

**Motivation.** `rotationFlicker` on the kick band, triggered densely enough
that back-to-back kicks (fills, rolls) produce overlapping flickers. Keeping
`modulo: 1` and `strengthThreshold` low means every kick lands; closely-spaced
kicks (<100ms apart) will stack via `compose: add`.

```json
{
  "tickers": [
    { "id": "kick-fill-flicker", "band": "kick", "modulo": 1, "strengthThreshold": 0.2 }
  ],
  "bindings": [
    {
      "tickerId": "kick-fill-flicker",
      "target": "rows",
      "rows": "all",
      "compose": "add",
      "sequence": {
        "actions": [
          { "path": "rotationFlicker", "params": { "magnitude": 3 }, "duration": 0.09, "easing": "linear" }
        ]
      }
    }
  ]
}
```

**Expected visual.** Steady kicks produce a single clean rotational twitch each; fills (rapid kicks) stack flickers into a sustained chaotic wobble.

---

## Notes on the current path registry

All recipes above compose entirely from paths registered in
`src/motion/paths.rs`: `slideStep`, `driftSpeed`, `glyphJitter`, `scalePulse`,
`rotationFlicker`, `cameraPanDelta`, `cameraZoomDeltaFocused`. No new paths
were required.
