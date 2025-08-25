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
#    python313Packages.uv
    uv
    php83
    php83Packages.composer
    yarn
    awscli2
    playwright
    playwright-test
sqlite
openssl
prisma
  ];
  GREETING = "Hi Haoran!";
  
  shellHook = ''
    echo $GREETING | cowsay
  '';
}
