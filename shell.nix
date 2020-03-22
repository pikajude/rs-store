with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "nix-store-shell";
  buildInputs = [ rustForDev.tools sqlite ];
  RUST_SRC_PATH = rustForDev.src;
}
