{
    inputs = {
        nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/*.tar.gz";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };
    outputs = attrs: let
        supportedSystems = [
            "aarch64-darwin"
            "aarch64-linux"
            "x86_64-darwin"
            "x86_64-linux"
        ];
        forEachSupportedSystem = f: attrs.nixpkgs.lib.genAttrs supportedSystems (system: f {
            pkgs = import attrs.nixpkgs {
                inherit system;
                overlays = [
                    attrs.rust-overlay.overlays.default # required for cargo-script
                ];
            };
        });
    in {
        devShells = forEachSupportedSystem ({ pkgs, ... }: pkgs.mkShell {
            pre-commit = pkgs.mkShell {
                packages = with pkgs; [
                    rust-bin.nightly.latest.default # nightly cargo, required to run the pre-commit script
                    cargo-deny
                ];
            };
        });
        packages = forEachSupportedSystem ({ pkgs, ... }: let
            manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        in {
            default = pkgs.rustPlatform.buildRustPackage {
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
    };
}
