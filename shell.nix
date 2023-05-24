{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
 	inputsFrom = [
    (pkgs.callPackage ./. { inherit pkgs; }).hoppy
  ];
  
  buildInputs = with pkgs; [
		rust-analyzer
		clippy
	];
}