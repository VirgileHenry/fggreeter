{ pkgs ? import <nixpkgs> {} }:
let
  greetd-src = builtins.fetchGit {
    url = "https://github.com/kennylevinsen/greetd";
    rev = "d6733e983ff7821c3044007d5555345c7553188f";
  };

  fakegreet = pkgs.rustPlatform.buildRustPackage {
    pname = "fakegreet";
    version = "0.10.3";
    src = greetd-src;
    cargoHash = "sha256-jRxVPnBeSv/f+Rx3u/CoXn2Hn8pD6yuABAvmcwf3A1o=";
    nativeBuildInputs = [ pkgs.perl ];
    cargoBuildFlags = [ "-p" "fakegreet" ];
    doCheck = false;
  };

  libs = [
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.vulkan-loader
    pkgs.udev
    pkgs.alsa-lib
  ];
in
pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.cage
    fakegreet
  ];
  buildInputs = libs;
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libs;
  UST_LOG="bevy_asset=debug,info";
}
