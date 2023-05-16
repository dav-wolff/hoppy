{
  description = "hoppy";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
      apps = pkgs.callPackage ./. {
        inherit pkgs;
      };
      platform = nixpkgs.rustPlatform;
    in {
      packages.x86_64-linux = rec {
        hoppy = apps.hoppy;
        hoppy-tester = apps.hoppy-tester;
        all = pkgs.symlinkJoin {
          name = "all";
          paths = with apps; [
            hoppy
            hoppy-tester
          ];
        };
        default = all;
      };
    };
}
