{
    inputs = {
        flake.url = "github:fenhl/flake";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "flake/nixpkgs";
        };
    };
    outputs = attrs: attrs.flake.lib {
        overlays = [
            attrs.rust-overlay.overlays.default # required for cargo-script
        ];
        devShells.pre-commit = { pkgs, ... }: {
            packages = with pkgs; [
                rust-bin.nightly.latest.default # nightly cargo, required to run the pre-commit script
                cargo-deny
            ];
        };
        packages.default = { pkgs, ... }: let
            manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        in pkgs.rustPlatform.buildRustPackage {
            inherit (manifest) version;
            pname = "molecule-db";
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
    };
}
