{
  description = "spotty - a minimal GPUI client for the Spotify Web API";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

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
