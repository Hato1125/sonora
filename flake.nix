{
  description = "spotty - a minimal native Spotify client";

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
          ];

          spotty = pkgs.rustPlatform.buildRustPackage {
            pname = "spotty";
            version = "0.1.0";

            src = ./.;

            cargoHash = "sha256-8GIkBbhaZwI21owOybq4cA+qsnT8oqs4Z+z24vvWRHo=";

            nativeBuildInputs = with pkgs; [
              pkg-config
              mold
              bintools
            ];

            buildInputs = runtimeLibraries;

            postFixup = ''
              patchelf \
                --add-rpath "${pkgs.lib.makeLibraryPath runtimeLibraries}" \
                "$out/bin/spotty"
            '';

            meta = {
              description = "A minimal native Spotify client built with GPUI";
              mainProgram = "spotty";
              platforms = pkgs.lib.platforms.linux;
            };
          };
        in
        {
          inherit spotty;
          default = spotty;
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
