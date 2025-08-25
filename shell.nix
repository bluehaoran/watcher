# save this as shell.nix
{ pkgs ? import <nixpkgs> {}}:

pkgs.mkShell {
  # packages = [ pkgs.hello ];
  packages = with pkgs; [
    cowsay
    git
    vim-full
    hello
    nodejs_22
    uv
    awscli2
    rustc
    cargo
    clippy
    rustup
    rustfmt
    sqlite
    openssl
    pkg-config
    sqlx-cli
  ];
  GREETING = "Hi Haoran!";
  
  shellHook = ''
    echo $GREETING | cowsay
  '';
}
