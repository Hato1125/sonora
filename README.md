<div align="center">

# Sonora

[![Build](https://img.shields.io/github/actions/workflow/status/nolight132/sonora/release.yml)](https://github.com/nolight132/sonora/actions/workflows/release.yml)
[![License](https://img.shields.io/github/license/nolight132/sonora)](./COPYING)

### A native music streaming client, built with Rust and GPUI.
Stream Spotify, YouTube Music, and local files all in one **native** app.
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

## Translations

Sonora ships these locales. Anything a locale is missing falls back to English at runtime, so a
partial translation is welcome — pick a language below and fill in what it lacks. Strings live in
`assets/i18n/<locale>/main.ftl`; `en-US` is the source of truth.

<!-- i18n:start -->

| Language | Translated | Coverage |
| --- | --- | --- |
| English (`en-US`) | 344/344 | 100% |
| Deutsch (`de`) | 344/344 | 100% |
| Français (`fr`) | 344/344 | 100% |
| Русский (`ru`) | 344/344 | 100% |
| Українська (`uk`) | 344/344 | 100% |
| Polski (`pl`) | 344/344 | 100% |

<!-- i18n:end -->

To add a language, create `assets/i18n/<locale>/main.ftl` and register it in
`crates/i18n/src/language.rs`. Regenerate the table with `scripts/i18n-coverage.py`.

## Credits

Sonora is built with the help of some incredible open-source projects, including:

- [Zed](https://github.com/zed-industries/zed) — a wonderful editor (~~ab~~)used by all core team members. Conveniently provides `gpui` — their native Rust rendering stack.
- [librespot](https://github.com/librespot-org/librespot) — Spotify playback and library integration.
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — certain YouTube ideas implemented in [ytmusic-rs](https://github.com/nolight132/ytmusic-rs). :)

## License

Copyright (C) 2026 Sonora Contributors.

Sonora is free software, released under the [GNU General Public License version
3 or later](COPYING).

Sonora is an unofficial client and is not affiliated with, endorsed by, or
sponsored by Spotify AB.

The binary also embeds the [Inter](https://github.com/rsms/inter) typeface (SIL
Open Font License 1.1) and the [Lucide](https://lucide.dev) icon set (ISC
License). `THIRD-PARTY.md` lists every bundled dependency.
