{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.luajit
    pkgs.luau
    pkgs.stylua
  ];
}
