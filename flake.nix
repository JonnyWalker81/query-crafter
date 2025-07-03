{
  description = "Query Crafter - TUI database client development environment";

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

        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            cargo-watch
            cargo-edit
            cargo-outdated
            cargo-audit
            cargo-nextest

            # Build tools
            pkg-config
            clang
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

          # Library paths for linking
          LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.xorg.libxcb
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXrandr
            pkgs.xorg.libXi
            pkgs.openssl
          ];
          
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.xorg.libxcb
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXrandr
            pkgs.xorg.libXi
            pkgs.openssl
          ];
          
          # PKG_CONFIG configuration
          PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" [
            pkgs.openssl.dev
            pkgs.xorg.libxcb.dev
            pkgs.xorg.libX11.dev
          ];
        };
      }
    );
}
