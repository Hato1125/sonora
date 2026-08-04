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
            alsa-lib
          ];

          gpuiVersion = "0.2.2";
          gpuiCrate = pkgs.fetchurl {
            url = "https://static.crates.io/crates/gpui/gpui-${gpuiVersion}.crate";
            hash = "sha256-l5tFz6bscjtvQjMJFaGzdpuTDQKy1QX5aX+MpgK+5wc=";
          };
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

            shellHook = ''
              stamp=vendor/gpui/.patched-${gpuiVersion}
              if [ ! -f "$stamp" ]; then
                echo "spotty: materialising patched gpui ${gpuiVersion}"
                rm -rf vendor/gpui
                mkdir -p vendor
                tar xzf ${gpuiCrate} -C vendor
                mv vendor/gpui-${gpuiVersion} vendor/gpui
                chmod -R u+w vendor/gpui
                for p in patches/*.patch; do
                  patch -s -p1 -d vendor/gpui < "$p" || exit 1
                done
                touch "$stamp"
              fi
            '';
          };
        }
      );
    };
}
