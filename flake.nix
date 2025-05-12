{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  outputs = inputs: let
    pkgs = import inputs.nixpkgs {system = "x86_64-linux";};
  in {
    devShells.${pkgs.system}.default = pkgs.mkShell {
      packages = with pkgs; [
        gobject-introspection
        gtk3
        gtk-layer-shell

        bubblewrap

        (with luajitPackages; [
          lua
          lgi
          #
          stylua
          lua-language-server
        ])
        libseccomp
      ];
    };
    packages.${pkgs.system} = {
      default = inputs.self.packages.${pkgs.system}.sobercookie;

      sobercookie = pkgs.symlinkJoin {
        name = "sobercookie";
        paths = with inputs.self.packages.${pkgs.system}; [
          sobercookie-cli
          sobercookie-launcher
        ];
      };

      sobercookie-cli = let
        dependencies = with pkgs; [
          coreutils
          flatpak
          bubblewrap
        ];
      in
        pkgs.symlinkJoin {
          name = "sobercookie-cli";
          paths = [
            (pkgs.writeShellScriptBin "sobercookie" (builtins.readFile ./sobercookie))
          ];
          buildInputs = with pkgs; [makeWrapper];
          postBuild = ''
            wrapProgram $out/bin/sobercookie \
               --set PATH ${pkgs.lib.makeBinPath dependencies}
          '';
        };

      sobercookie-launcher = pkgs.stdenv.mkDerivation {
        name = "sobercookie-launcher";
        src = ./.;

        buildInputs = with pkgs; [luajit];
        nativeBuildInputs = with pkgs; [
          makeWrapper
          pkg-config
          wrapGAppsHook
          gobject-introspection
        ];

        propogatedBuildInputs = with pkgs; [gtk3];

        installPhase = ''
          mkdir -p $out/bin
          cp launcher.lua $out/bin/sobercookie-launcher

          mkdir -p $out/share/applications
          cp sobercookie-launcher.desktop $out/share/applications
        '';

        postInstall = ''
          chmod +x $out/bin/sobercookie-launcher
        '';

        preFixup = let
          inherit (pkgs) luajit;
          inherit (pkgs.luajitPackages) lgi;
        in ''
          gappsWrapperArgs+=(
            --prefix LUA_PATH : "./?.lua;${lgi}/share/lua/5.1/?.lua;${lgi}/share/lua/5.1/?/init.lua;${luajit}/share/lua/5.1/\?.lua;${luajit}/share/lua/5.1/?/init.lua"
            --prefix LUA_CPATH : "./?.so;${lgi}/lib/lua/5.1/?.so;${luajit}/lib/lua/5.1/?.so;${luajit}/lib/lua/5.1/loadall.so"
            --set PATH ${pkgs.lib.makeBinPath [inputs.self.packages.${pkgs.system}.sobercookie-cli]}
          )
        '';
      };
    };
  };
}
