let
  name = "glif";
  description = "glyph editor";
in {
  inherit name description;

  inputs = {
    nixpkgs.url      = github:nixos/nixpkgs/release-22.05;
    utils.url        = github:numtide/flake-utils;
    rust-overlay.url = github:oxalica/rust-overlay;
    naersk.url       = github:nix-community/naersk;

    # Used for shell.nix
    flake-compat = {
      url = github:edolstra/flake-compat;
      flake = false;
    };
  };

  outputs = { self, nixpkgs, rust-overlay, utils, naersk, ... } @ inputs: let
      overlays = [ rust-overlay.overlays.default ];
      # Our supported systems are the same supported systems as the Rust binaries
      systems = builtins.attrNames inputs.rust-overlay.packages;
    in utils.lib.eachSystem systems (system:
      let
        pkgs = import nixpkgs { inherit overlays system; };
        rust_channel = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        naersk-lib = naersk.lib."${system}".override {
          cargo = rust_channel;
          rustc = rust_channel;
        };
      in {
        defaultPackage = naersk-lib.buildPackage {
          pname = name;
          root = ./.;
        };

        devShells.default = pkgs.mkShell {
          inherit name description;
          buildInputs = with pkgs; [
            # all of rust unstable
            rust_channel
            rust-analyzer
            cargo
            # fast linking
            lld
            pkg-config
            glibc
          ];

          LD_LIBRARY_PATH = "${with pkgs; lib.makeLibraryPath ([ pkg-config ])}:$LD_LIBRARY_PATH";

          # for rust-analyzer; the target dir of the compiler for the project
          OUT_DIR = "./target";
          # don't warn for dead code, unused imports or unused variables
          RUSTFLAGS = "-A dead_code -A unused_imports -A unused_variables";
          # force cross compilation when there is potential for it
          CARGO_FEATURE_FORCE_CROSS = "true";
        };

        # For compatibility with older versions of the `nix` binary
        devShell = self.devShells.${system}.default;
      });
}
