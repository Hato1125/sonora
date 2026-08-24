{
  description = "Sonora - a native music streaming client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forEachSystem = fn: nixpkgs.lib.genAttrs systems (system: fn nixpkgs.legacyPackages.${system});

      release = {
        version = "0.17.0";
        assets = {
          x86_64-linux = {
            target = "x86_64-unknown-linux-gnu";
            hash = "sha256-t1leoJYgvlarFT5z9MM9LqJFCBOjTuarR7btOHtwRwA=";
          };
          aarch64-linux = {
            target = "aarch64-unknown-linux-gnu";
            hash = "sha256-7yHOg/lCxTugo1j0IZ+/uSeCM3x26Lj6wFSVR7XGazE=";
          };
        };
      };
    in
    {
      packages = forEachSystem (
        pkgs:
        let
          runtimeLibraries = with pkgs; [
            vulkan-loader
            wayland
            libxkbcommon
            libxcb
            libx11
            libxcursor
            libxi
            fontconfig
            freetype
            alsa-lib
            dbus
          ];

          asset = release.assets.${pkgs.stdenv.hostPlatform.system};

          sonora-bin = pkgs.stdenv.mkDerivation {
            pname = "sonora-bin";
            inherit (release) version;

            src = pkgs.fetchurl {
              url = "https://github.com/nolight132/sonora/releases/download/v${release.version}/sonora-v${release.version}-${asset.target}";
              inherit (asset) hash;
            };

            dontUnpack = true;
            dontStrip = true;

            installPhase = ''
              runHook preInstall
              install -Dm755 "$src" "$out/bin/sonora"
              install -Dm444 ${./assets/linux/sonora.desktop} \
                "$out/share/applications/sonora.desktop"
              install -Dm444 ${./assets/linux/sonora.svg} \
                "$out/share/icons/hicolor/scalable/apps/sonora.svg"
              for icon in ${./assets/linux/icons}/hicolor/*/apps/sonora.png; do
                size="$(basename "$(dirname "$(dirname "$icon")")")"
                install -Dm444 "$icon" \
                  "$out/share/icons/hicolor/$size/apps/sonora.png"
              done
              install -Dm444 ${./COPYING} "$out/share/licenses/sonora/LICENSE"
              install -Dm444 ${./THIRD-PARTY.md} "$out/share/licenses/sonora/THIRD-PARTY.md"
              install -Dm444 ${./assets/fonts/LICENSE.txt} \
                "$out/share/licenses/sonora/LICENSE.Inter"
              install -Dm444 ${./assets/icons/LICENSE} \
                "$out/share/licenses/sonora/LICENSE.Lucide"
              runHook postInstall
            '';

            postFixup = ''
              patchelf \
                --set-interpreter "${pkgs.stdenv.cc.bintools.dynamicLinker}" \
                --add-rpath "${
                  pkgs.lib.makeLibraryPath (runtimeLibraries ++ [ pkgs.stdenv.cc.cc.lib ])
                }" \
                "$out/bin/sonora"
            '';

            meta = {
              description = "A native music streaming client, built with Rust and GPUI";
              mainProgram = "sonora";
              license = with pkgs.lib.licenses; [
                gpl3Plus
                ofl
                isc
              ];
              platforms = pkgs.lib.platforms.linux;
            };
          };
        in
        {
          inherit sonora-bin;
          sonora = sonora-bin;
          default = sonora-bin;
        }
      );

      devShells = forEachSystem (
        pkgs:
        let
          runtimeLibraries = with pkgs; [
            vulkan-loader
            wayland
            libxkbcommon
            libxcb
            libx11
            libxcursor
            libxi
            fontconfig
            freetype
            alsa-lib
            dbus
          ];
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              mold
              pkg-config
              rustc
              rust-analyzer
              rustfmt
              sccache
            ];

            buildInputs = runtimeLibraries;

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibraries;
          };
        }
      );
    };
}
