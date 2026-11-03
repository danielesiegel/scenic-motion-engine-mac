{
  description = "scenic-motion-engine — reproducible Rust build (macOS)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachSystem [
      "aarch64-darwin"
      "x86_64-darwin"
    ] (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Pin Rust >= 1.75 (stable channel).
        rustToolchain = pkgs.rust-bin.stable."1.75.0".default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" "rust-analyzer" ];
          targets = [ "aarch64-apple-darwin" "x86_64-apple-darwin" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
        ];

        buildInputs = with pkgs; [
          ffmpeg
          libxkbcommon
        ];
      in {
        packages.default = rustPlatform.buildRustPackage {
          pname = "scenic-motion-engine";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          inherit nativeBuildInputs buildInputs;
          doCheck = false;
          meta = with pkgs.lib; {
            description = "scenic motion engine";
            platforms = [ "aarch64-darwin" "x86_64-darwin" ];
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [ rustToolchain ] ++ nativeBuildInputs ++ buildInputs
            ++ (with pkgs; [ ffmpeg ]);
          shellHook = ''
            echo "scenic-motion-engine devShell (${system})"
            echo "rustc: $(rustc --version)"
          '';
        };
      });
}
