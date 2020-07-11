with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "shell";
  buildInputs = [ sqlite pkg-config xz ];
}
