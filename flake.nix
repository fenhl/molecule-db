{
    inputs = {
        nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/*.tar.gz";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };
    outputs = attrs: attrs.flake-utils.lib.eachDefaultSystem (system: let
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        pkgs = import attrs.nixpkgs {
            inherit system;
            overlays = [
                attrs.rust-overlay.overlays.default # required for cargo-script
            ];
        };
    in {
        devShells.pre-commit = pkgs.mkShell {
            packages = with pkgs; [
                rust-bin.nightly.latest.default # nightly cargo, required to run the pre-commit script
                cargo-deny
            ];
        };
        packages.default = pkgs.rustPlatform.buildRustPackage {
            pname = "molecule-db";
            version = manifest.version;
            buildFeatures = [
                "night"
                "nixos"
            ];
            cargoLock = {
                allowBuiltinFetchGit = true; # allows omitting cargoLock.outputHashes
                lockFile = ./Cargo.lock;
            };
            src = ./.;
        };
    });
}
