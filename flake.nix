{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        version = "1.0.0";
        pkgs = (import nixpkgs) { inherit system; };
        nativeBuildInputs = with pkgs; [ cmake pkg-config rustc cargo ];
        buildInputs = [ ];
        mkPackage = { name, buildInputs ? [ ] }: pkgs.rustPlatform.buildRustPackage {
          cargoBuildOptions = "--package ${name}";
          pname = name;
          inherit version;
          inherit buildInputs;
          inherit nativeBuildInputs;
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "tree-sitter-ledger-0.0.1" = "sha256-Yd8xPsUN8uidfz5d9xXA29BdxhFJ+BXkDDlyFgkGW3o=";
            };
          };
          src = ./.;
          postInstall = "
            cp -r target/*/release/share $out/share
          ";
        };
      in
      rec {
        formatter = pkgs.nixpkgs-fmt;
        packages.ledger-beautifier = mkPackage { name = "ledger-beautifier"; };
        packages.default = packages.ledger-beautifier;
        apps = rec {
          ledger-beautifier = { type = "app"; program = "${packages.default}/bin/ledger-beautifier"; };
          default = ledger-beautifier;
        };
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo busybox clang-tools fzf ];
          inherit buildInputs;
        };
      }
    );
}
