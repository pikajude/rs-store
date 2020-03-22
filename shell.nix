with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "nix-store-shell";
  buildInputs = [ rustForDev.tools ];
  RUST_SRC_PATH = rustForDev.src;
}
