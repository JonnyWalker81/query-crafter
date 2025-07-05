# Shell environment for query-crafter
# This provides the pre-built binary in a nix-shell
{ pkgs ? import <nixpkgs> {} }:

let
  query-crafter = pkgs.callPackage ./query-crafter.nix { };
in
pkgs.mkShell {
  buildInputs = [ query-crafter ];
  
  shellHook = ''
    echo "Query Crafter shell environment"
    echo "==============================="
    echo "query-crafter is now available in your PATH"
    echo ""
    echo "Run 'query-crafter --help' to get started"
  '';
}