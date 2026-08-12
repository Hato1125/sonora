<div align="center">

# Sonora

[![Build](https://img.shields.io/github/actions/workflow/status/nolight132/sonora/release.yml)](https://github.com/nolight132/sonora/actions/workflows/release.yml)
[![License](https://img.shields.io/github/license/nolight132/sonora)](./COPYING)

### A native music streaming client for Spotify, YouTube, and local music, built with Rust and GPUI.
This project would not be possible without [librespot](https://github.com/librespot-org/librespot).
</div>

<div align="center">
    <table>
      <tr>
        <td colspan="2">
          <img width="1613" height="976" alt="image" src="https://github.com/user-attachments/assets/55204aaa-bf53-434b-a713-39c5ef3436e1" />
        </td>
      </tr>
      <tr>
        <td width="50%">
          <img width="1618" height="976" alt="image" src="https://github.com/user-attachments/assets/51ff66c7-8a20-4e0f-9ebe-8ed4865c88a6" />
        </td>
        <td width="50%">
          <img width="1618" height="976" alt="image" src="https://github.com/user-attachments/assets/06729fc8-6281-4cb0-a85c-f8c22921b4bd" />
        </td>
      </tr>
    </table>
</div>
<div align="center">
    <sub>
      Adaptive themes are optional. Everything is (or will be) customizable.
    </sub>
</div>

## Features
- **Spotify**, **YouTube**, and local playback.
- Library management within supported providers.
- Gapless playback (Spotify).
- Synced lyrics.
- Cross-platform support.
- Audio normalization.
- Custom themes.
- Offline local playback.

## Install

### macOS

```sh
brew install --cask nolight132/tap/sonora
```
After installing (thanks Apple):

```sh
xattr -dr com.apple.quarantine /Applications/Sonora.app
```

### Linux

```sh
AUR, COPR, .deb coming soon.
```

### Nix
Just use the flake in the project root.

```sh
inputs.sonora.packages.${system}.sonora-bin
```

`sonora-bin` tracks the latest tagged release.

### Windows
Download the latest `.exe` for your architecture from [Releases](https://github.com/nolight132/sonora/releases/latest). There isn't a proper Windows installer yet.

## Credits

Sonora is built with the help of some incredible open-source projects, including:

- [Zed](https://github.com/zed-industries/zed) — a wonderful editor (~~ab~~)used by all core team members. Conveniently provides `gpui` — their native Rust rendering stack.
- [librespot](https://github.com/librespot-org/librespot) — Spotify playback and library integration.
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — certain YouTube ideas implemented in [ytmusic-rs](https://github.com/nolight132/ytmusic-rs). :)

## License

Copyright (C) 2026 nolight132.

Sonora is free software, released under the [GNU General Public License version
3 or later](COPYING).

Sonora is an unofficial client and is not affiliated with, endorsed by, or
sponsored by Spotify AB.

The binary also embeds the [Inter](https://github.com/rsms/inter) typeface (SIL
Open Font License 1.1) and the [Lucide](https://lucide.dev) icon set (ISC
License). `THIRD-PARTY.md` lists every bundled dependency.
