{
  description = "Query Crafter - TUI database client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Nightly Rust toolchain
        # rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
        #   extensions = [
        #     "rust-src"
        #     "rust-analyzer"
        #   ];
        # };

        # Build dependencies
        nativeBuildInputs = with pkgs; [
          # Rust toolchain
          # rustToolchain
          # cargo-watch
          # cargo-edit
          # cargo-outdated
          # cargo-audit
          # cargo-nextest

          # Build tools
          pkg-config
          clang
          python3 # Required by xcb build script
        ];

        buildInputs = with pkgs; [
          # OpenSSL
          openssl
          openssl.dev

          # X11 dependencies (for clipboard support)
          xorg.libxcb
          xorg.libxcb.dev
          xorg.libX11
          xorg.libX11.dev
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi

          # Runtime dependencies
          nodejs_20
          python3
          awscli2

          # Development tools
          git
          ripgrep
          fd
          tokei
          hyperfine
          just
          bacon

          # Database clients (for testing)
          postgresql
          sqlite
        ];

        # Shared library paths
        libPath = pkgs.lib.makeLibraryPath [
          pkgs.xorg.libxcb
          pkgs.xorg.libX11
          pkgs.xorg.libXcursor
          pkgs.xorg.libXrandr
          pkgs.xorg.libXi
          pkgs.openssl
        ];

        # PKG_CONFIG path
        pkgConfigPath = pkgs.lib.makeSearchPath "lib/pkgconfig" [
          pkgs.openssl.dev
          pkgs.xorg.libxcb.dev
          pkgs.xorg.libX11.dev
        ];

      in
      {
        # Development shell
        devShells = {
          default = pkgs.mkShell {
            inherit nativeBuildInputs buildInputs;

            shellHook = ''
              echo "Query Crafter Development Environment"
              echo "====================================="
              echo "Rust: $(rustc --version)"
              echo "Cargo: $(cargo --version)"
              echo "AWS CLI: $(aws --version)"
              echo ""
              echo "Available commands:"
              echo "  cargo build              - Build the project"
              echo "  cargo run                - Run query-crafter"
              echo "  cargo test               - Run tests"
              echo "  cargo watch -x run       - Run with auto-reload"
              echo "  cargo nextest run        - Run tests with nextest"
              echo "  bacon                    - Run bacon for continuous checking"
              echo ""
            '';

            # Environment variables
            RUST_BACKTRACE = 1;
            RUST_LOG = "query_crafter=debug";
            LIBRARY_PATH = libPath;
            LD_LIBRARY_PATH = libPath;
            PKG_CONFIG_PATH = pkgConfigPath;
          };
        };

        # Package output
        # packages = {
        #   default = pkgs.rustPlatform.buildRustPackage {
        #     pname = "query-crafter";
        #     version = "0.1.0";

        #     src = ./.;

        #     cargoLock = {
        #       lockFile = ./Cargo.lock;
        #     };

        #     inherit nativeBuildInputs buildInputs;

        #     # Set library paths for build
        #     preBuild = ''
        #       export LIBRARY_PATH="${libPath}"
        #       export LD_LIBRARY_PATH="${libPath}"
        #       export PKG_CONFIG_PATH="${pkgConfigPath}"
        #     '';

        #     meta = with pkgs.lib; {
        #       description = "A modern TUI database client with VIM keybindings";
        #       homepage = "https://github.com/yourusername/query-crafter";
        #       license = licenses.mit;
        #       maintainers = [ ];
        #     };
        #   };
        # };

        # # App output for `nix run`
        # apps = {
        #   default = flake-utils.lib.mkApp {
        #     drv = self.packages.${system}.default;
        #   };
        # };
      }
    );
}
