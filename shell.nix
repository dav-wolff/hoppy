{ pkgs ? import <nixpkgs> { } }:
let
	apps = pkgs.callPackage ./. {
		inherit pkgs;
	};
in pkgs.mkShell {
	inputsFrom = [
		apps.hoppy
	];
	
	buildInputs = with pkgs; [
		rust-analyzer
		clippy
		apps.hoppy
		apps.hoppy-tester
	];
}