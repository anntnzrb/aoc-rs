{
  description = "Advent of Code Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts/main";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix/main";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];
      perSystem =
        { pkgs, system, ... }:
        let
          rustToolchain = inputs.fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-+xUr6Al1wZSqX8aEsq7DpuFOK4Ve/i+qTl+K5uWQ9GM=";
          };
          cargoDeny = pkgs.rustPlatform.buildRustPackage {
            pname = "cargo-deny";
            version = "0.20.2";
            src = pkgs.fetchCrate {
              pname = "cargo-deny";
              version = "0.20.2";
              hash = "sha256-MYE/YO5v46VxatY1/Asw+hXu9GKMdDKhnXvIGuYTCQc=";
            };
            cargoHash = "sha256-Zb6vQCnhhhL9Ducn9eh5P8Gfopl0lQPTXWW8Q0Y5xBQ=";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.zstd ];
            env.ZSTD_SYS_USE_PKG_CONFIG = true;
            # Upstream tests require network access outside the build sandbox.
            doCheck = false;
          };
        in
        {
          formatter = pkgs.nixfmt;
          devShells.default = pkgs.mkShell {
            name = "aoc-rs-dev";
            packages = [
              rustToolchain
              pkgs.cargo-nextest
              cargoDeny
              pkgs.cargo-machete
              pkgs.uv
              pkgs.zizmor
              # Workflow operations: HTTP for fetch/puzzle/submit, the task runner,
              # and GNU time for benchmark CPU and RSS.
              pkgs.curl
              pkgs.just
              pkgs.time
            ]
            # Benchmark work counts and CPU pinning exist only on Linux.
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              pkgs.perf
              pkgs.util-linux
            ];
          };
        };
    };
}
