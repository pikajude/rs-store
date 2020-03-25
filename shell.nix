with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "nix-store-shell";
  buildInputs = [ rustc rls cargo clippy rustfmt sqlite ];
}
