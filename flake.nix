{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        meta = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
        inherit (meta) name version;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = (with pkgs; [
            rust
            git
          ]);
        };
        packages.default = self.packages.${system}.just-lsp;
        packages.just-lsp = pkgs.rustPlatform.buildRustPackage {
          pname = name;
          inherit version;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          release = true;
        };
      });
}
