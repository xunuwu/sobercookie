{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  packages = with pkgs; [
    gobject-introspection
    gtk3
    gtk-layer-shell
    (with luajitPackages; [
      lua
      lgi
      luaposix
      #
      stylua
      lua-language-server
    ])
  ];
}
