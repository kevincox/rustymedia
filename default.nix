with import <nixpkgs> {}; let
in rec {
	out = rustPlatform.buildRustPackage {
		name = "rustymedia";
		meta = {
			description = "RustyMedia Server";
			homepage = https://kevincox.ca;
		};
		
		cargoSha256 = null;
		src = builtins.filterSource (name: type:
			(lib.hasPrefix (toString ./src) name) ||
			(lib.hasPrefix (toString ./Cargo) name)) ./.;
		
		FFMPEG_BINARY = "${ffmpeg}/bin/ffmpeg";
		FFPROBE_BINARY = "${ffmpeg}/bin/ffprobe";
		
		doCheck = false;

		# Work around https://github.com/NixOS/nixpkgs/pull/34034
		postUnpack = ''
			eval "$cargoDepsHook"
			unpackFile "$cargoDeps"
			cargoDepsCopy=$(stripHash $(basename $cargoDeps))
			chmod -R +w "$cargoDepsCopy"
			mkdir -p .cargo
			cat >.cargo/config <<-EOF
				[source.crates-io]
				registry = 'https://github.com/rust-lang/crates.io-index'
				replace-with = 'vendored-sources'

				[source."https://github.com/kevincox/lru-cache.git"]
				git = "https://github.com/kevincox/lru-cache.git"
				branch = "entry-api"
				replace-with = "vendored-sources"

				[source.vendored-sources]
				directory = '$(pwd)/$cargoDepsCopy'
			EOF
			unset cargoDepsCopy
			export RUST_LOG=warn
		'';
	};
}
