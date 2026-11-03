# website

Static project site for `scenic-motion-engine-mac`. Pure HTML + CSS, no JS, no build step.

## Serve locally

```sh
python3 -m http.server --directory website 8000
```

Then open http://localhost:8000.

## Hosting

Static-only — can be hosted on GitHub Pages. In repository Settings -> Pages, set:

- Source: `Deploy from a branch`
- Branch: `main` / folder: `/website`

Note: the screenshot is referenced as `../docs/screenshots/app.png`. If serving `website/` as the Pages root breaks that relative path, either copy `app.png` into `website/` and update `index.html`, or publish from the repo root and link to `/website/`.
