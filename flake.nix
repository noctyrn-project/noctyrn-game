{
  description = "Fearlyss FPS Game - Bevy Engine";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
            cmake
            mold
          ];

          buildInputs = with pkgs; [
            # Bevy dependencies
            udev
            alsa-lib
            vulkan-loader
            libx11
            libxcursor
            libxi
            libxrandr
            wayland
            libxkbcommon

            # Additional
            openssl
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

          RUST_BACKTRACE = 1;
        };
      });
}
