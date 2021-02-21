let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  nixos2003 = import <nixos-2003> {};
in
  nixpkgs.stdenv.mkDerivation {
    name = "ffxdghack";
    buildInputs = (with nixpkgs; [
      rustup
    ]);
  }

