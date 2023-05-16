{ pkgs ? import <nixpkgs> { } }:
let
  buildWorkspaceMember = path: let
    manifest = (pkgs.lib.importTOML ./${path}/Cargo.toml).package;
  in pkgs.rustPlatform.buildRustPackage rec {
    pname = manifest.name;
    version = manifest.version;
    
    cargoLock.lockFile = ./Cargo.lock;
    src = pkgs.lib.cleanSource ./.;
    cargoBuildFlags = "-p ${path}";
    
    buildInputs = [
      pkgs.systemd
    ];
    nativeBuildInputs = [
      pkgs.pkg-config
    ];
  };
  manifest = (pkgs.lib.importTOML ./Cargo.toml);
in {
  hoppy = buildWorkspaceMember "hoppy";

  hoppy-tester = buildWorkspaceMember "hoppy-tester";
}
