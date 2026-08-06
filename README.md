# sonora

A minimal native Spotify client built with Rust and GPUI.

## Arch Linux / CachyOS

Install the build and runtime dependencies:

```sh
sudo pacman -S --needed base-devel rust pkgconf alsa-lib fontconfig freetype2 \
  libx11 libxcb libxcursor libxi libxkbcommon libxkbcommon-x11 wayland \
  vulkan-icd-loader
```

You also need a Vulkan driver for your GPU, such as `vulkan-radeon`,
`vulkan-intel`, or the Vulkan support included with the NVIDIA driver.

Build and run:

```sh
cargo run --locked --package sonora
```

For an optimized build:

```sh
cargo build --release --locked --package sonora
./target/release/sonora
```

The first build downloads and compiles GPUI and the other Rust dependencies,
so it can take a few minutes.

## Nix

```sh
nix run
```
