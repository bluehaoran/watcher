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
    openssl
    
    # Playwright/Chromium dependencies
    glib
    gobject-introspection
    nspr
    nss
    dbus
    atk
    at-spi2-atk
    expat
    at-spi2-core
    xorg.libX11
    xorg.libXcomposite
    xorg.libXdamage
    xorg.libXext
    xorg.libXfixes
    xorg.libXrandr
    mesa
    xorg.libxcb
    libxkbcommon
    systemd
    alsa-lib
  ];
  GREETING = "Hi Haoran!";
  
  shellHook = ''
    echo $GREETING | cowsay
  '';
}
