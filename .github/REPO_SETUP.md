# Repository Setup

One-time configuration steps for the `scenic-motion-engine-mac` repository on GitHub.

## 1. Enable GitHub Pages

The `pages.yml` workflow publishes the `website/` directory on every push to `main` (and on manual dispatch), but Pages must be switched on first.

1. Open the repo on GitHub.
2. Go to **Settings → Pages**.
3. Under **Build and deployment → Source**, select **GitHub Actions**.
4. Save. The next successful run of `.github/workflows/pages.yml` will publish the site; the resulting URL will appear on the Pages settings page and in the workflow run summary.

No branch selection is needed when the source is set to GitHub Actions.

## 2. Set Repository Topics

Topics make the repo discoverable and signal its tech stack. Set them under **Settings → General → Topics** (or click the gear icon next to **About** on the repo home page).

Recommended topics:

- `rust`
- `macos`
- `audio-reactive`
- `motion-graphics`
- `wgpu`
- `cosmic-text`
- `glyphon`
- `cpal`
- `egui`
- `visualizer`

## 3. Enable Discussions

GitHub Discussions provides a forum tab for questions, ideas, and showcase posts separate from issues.

1. Go to **Settings → General**.
2. Scroll to the **Features** section.
3. Tick the **Discussions** checkbox.
4. Click **Set up discussions** (or just save) — GitHub seeds the default categories (Announcements, General, Ideas, Q&A, Show and tell).

The **Discussions** tab will then appear on the repo home page alongside **Code**, **Issues**, and **Pull requests**.
