{ pkgs ? import <nixpkgs> {} }:

let
  version = "0.1.0"; # Update this to match your release version
  
  # Define the binary for each platform
  sources = {
    x86_64-linux = {
      url = "https://github.com/JonnyWalker81/query-crafter/releases/download/v${version}/query-crafter-v${version}-linux-x86_64.tar.gz";
      sha256 = ""; # Fill in with actual sha256 after first release
    };
    aarch64-linux = {
      url = "https://github.com/JonnyWalker81/query-crafter/releases/download/v${version}/query-crafter-v${version}-linux-arm64.tar.gz";
      sha256 = ""; # Fill in with actual sha256 after first release
    };
    x86_64-darwin = {
      url = "https://github.com/JonnyWalker81/query-crafter/releases/download/v${version}/query-crafter-v${version}-macos-x86_64.tar.gz";
      sha256 = ""; # Fill in with actual sha256 after first release
    };
    aarch64-darwin = {
      url = "https://github.com/JonnyWalker81/query-crafter/releases/download/v${version}/query-crafter-v${version}-macos-arm64.tar.gz";
      sha256 = ""; # Fill in with actual sha256 after first release
    };
  };
  
  source = sources.${pkgs.stdenv.hostPlatform.system} or (throw "Unsupported system: ${pkgs.stdenv.hostPlatform.system}");
  
in pkgs.stdenv.mkDerivation rec {
  pname = "query-crafter";
  inherit version;
  
  src = pkgs.fetchurl {
    inherit (source) url sha256;
  };
  
  # No build phase needed for pre-built binary
  dontBuild = true;
  
  # Use autoPatchelfHook to automatically patch the binary
  nativeBuildInputs = with pkgs; [
    autoPatchelfHook
  ];
  
  # Runtime dependencies
  buildInputs = with pkgs; [
    stdenv.cc.cc.lib
    openssl
  ];
  
  installPhase = ''
    runHook preInstall
    
    # Extract the tarball
    tar -xzf $src
    
    # Install the binary
    install -D -m755 query-crafter $out/bin/query-crafter
    
    runHook postInstall
  '';
  
  meta = with pkgs.lib; {
    description = "TUI for interacting with databases";
    homepage = "https://github.com/JonnyWalker81/query-crafter";
    license = licenses.mit; # Update this to match your actual license
    maintainers = with maintainers; [ ]; # Add maintainer info if desired
    platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    mainProgram = "query-crafter";
  };
}