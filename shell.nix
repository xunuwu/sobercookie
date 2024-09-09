{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  packages = with pkgs; [
    gobject-introspection
    gtk3
    gtk-layer-shell
    (with luajitPackages; [
      lua
      lgi
      #
      stylua
      lua-language-server
    ])

    # bash
    jq
    libnotify
  ];
}
