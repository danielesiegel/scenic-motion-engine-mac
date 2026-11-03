# Packaging

This directory contains distribution packaging artifacts for Scenic Motion Engine.

## Homebrew Cask

`homebrew/scenic-motion-engine.rb` is a [Homebrew Cask](https://docs.brew.sh/Cask-Cookbook) formula that installs the official `.app` bundle shipped as a DMG on the project's [GitHub Releases](https://github.com/danielesiegel/scenic-motion-engine-mac/releases) page.

The cask points at the `latest` release asset:

```
https://github.com/danielesiegel/scenic-motion-engine-mac/releases/latest/download/scenic-motion-engine.dmg
```

### Installing (end user)

Once the tap repo is published, users will install via:

```sh
brew install --cask danielesiegel/tap/scenic-motion-engine
```

### Publishing the formula (maintainer)

The cask file in this repo is the canonical source, but Homebrew needs a dedicated
**tap repository** (named `homebrew-tap`) to host it. The tap repo must:

- Be named `homebrew-tap` under the `danielesiegel` GitHub user/org
- Contain `Casks/scenic-motion-engine.rb` (a copy of this file)
- Be public so `brew tap` can clone it

See the official Homebrew documentation on taps:

- [How to Create and Maintain a Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)

After the first signed release ships, replace the `:no_check` sha256 placeholder with the real checksum:

```sh
shasum -a 256 scenic-motion-engine.dmg
```

and copy the cask into the tap repo under `Casks/`.
