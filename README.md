<div align="center">

# Sonora

[![Build](https://img.shields.io/github/actions/workflow/status/nolight132/sonora/release.yml)](https://github.com/nolight132/sonora/actions/workflows/release.yml)
[![License](https://img.shields.io/github/license/nolight132/sonora)](./LICENSE)

### A minimal native Spotify client built with Rust and GPUI.
This project would not be possible without [librespot](https://github.com/librespot-org/librespot).
</div>

<div align="center">
    <table>
      <tr>
        <td colspan="2">
          <img width="1532" height="967" alt="image" src="https://github.com/user-attachments/assets/90bad3c1-6ad1-4354-930f-511ea8480d74" />
        </td>
      </tr>
      <tr>
        <td width="50%">
          <img width="100%" alt="Sonora" src="https://github.com/user-attachments/assets/619b5d7a-08c9-4367-a28b-0f9485865c82" />
        </td>
        <td width="50%">
          <img width="100%" alt="Sonora" src="https://github.com/user-attachments/assets/7aca84c2-4610-4a78-af21-8adb3276a4a1" />
        </td>
      </tr>
    </table>
</div>
<div align="center">
    <sub>
      Adaptive themes are optional. Everything is (or will be) customizable.
    </sub>
</div>





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

## License

Copyright (C) 2026 nolight132.

Sonora is free software, released under the [GNU General Public License version
3 or later](LICENSE).

Sonora is an unofficial client and is not affiliated with, endorsed by, or
sponsored by Spotify AB.

The binary also embeds the [Inter](https://github.com/rsms/inter) typeface (SIL
Open Font License 1.1) and the [Lucide](https://lucide.dev) icon set (ISC
License). `THIRD-PARTY.md` lists every bundled dependency.
