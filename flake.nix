{
  description = "Terminal UI to list, browse and run APIs defined with OpenAPI v3.0 spec";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    systems.url = "github:nix-systems/default-linux";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    systems,
    flake-utils,
    ...
  } @ inputs: let
    inherit (nixpkgs) lib;
    eachSystem = lib.genAttrs (import systems);
    pkgsFor = eachSystem (
      system: let
        systemPkgs = import nixpkgs {
          inherit system;
          overlays = [
            fenix.overlays.default
          ];
        };
      in
        systemPkgs
        // {
          myPackage = import ./nix/default.nix {
            inherit system;
            pkgs = systemPkgs;
            lockFile = ./Cargo.lock;
            fenix = fenix;
          };
        }
    );
  in {
    packages = eachSystem (system: {
      openapi-tui = pkgsFor.${system}.myPackage;
    });

    checks = eachSystem (system: self.packages.${system});

    formatter = eachSystem (system: pkgsFor.${system}.alejandra);
  };
}
