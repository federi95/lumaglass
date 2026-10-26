{
  description = "Lumaglass Rust/SDL2 development and packaging environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
      project = builtins.fromJSON (builtins.readFile ./package.json);
      pnpmVersion = nixpkgs.lib.removePrefix "pnpm@" project.packageManager;
    in
    {
      devShells = forEachSystem (
        pkgs:
        let
          # Match the native CI toolchain and include the TV's soft-float Rust std.
          rust = pkgs.rust-bin.stable."1.93.0".minimal.override {
            extensions = [
              "clippy"
              "rustfmt"
            ];
            targets = [ "armv7-unknown-linux-gnueabi" ];
          };
          # Pin pnpm independently of nixpkgs so local, Nix and CI use the same
          # security patch even before it reaches the nixpkgs snapshot.
          pnpm = pkgs.pnpm_10.override {
            version = pnpmVersion;
            hash = "sha256-foqUYQjnuqsRnlHSfCTWF4tGVFQiZD28keNOLx7jLg8=";
          };
        in
        {
          default = pkgs.mkShell ({
            name = "lumaglass";
            packages = [
              pkgs.nodejs_24
              pnpm
              rust
              pkgs.pkg-config
              pkgs.SDL2
              pkgs.woff2
              (pkgs.python3.withPackages (python: [ python.fonttools ]))
              pkgs.cmake
              pkgs.git
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              pkgs.libGL
              pkgs.mesa
              pkgs.xvfb-run
            ];
          });
        }
      );

      formatter = forEachSystem (
        pkgs:
        pkgs.writeShellApplication {
          name = "lumaglass-nixfmt";
          runtimeInputs = [ pkgs.nixfmt ];
          text = ''
            exec nixfmt "$@" flake.nix
          '';
        }
      );
    };
}
