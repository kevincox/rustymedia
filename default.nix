with import <nixpkgs> {}; let
in rec {
	out = rustPlatform.buildRustPackage {
		name = "rustymedia";
		meta = {
			description = "RustyMedia Server";
			homepage = https://kevincox.ca;
		};
		
		cargoSha256 = "";
		src = builtins.filterSource (name: type:
			(lib.hasPrefix (toString ./src) name) ||
			(lib.hasPrefix (toString ./Cargo) name)) ./.;
		
		FFMPEG_BINARY = "${ffmpeg}/bin/ffmpeg";
		
		doCheck = false;
	};
}
