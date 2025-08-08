{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      nixpkgs,
      systems,
      rust-overlay,
      ...
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (
            toolchain:
            toolchain.default.override {
              extensions = [
                "llvm-tools-preview"
                "rust-src"
                "rust-analyzer"
              ];
            }
          );
        in
        {
          default = (pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }) {
            packages = with pkgs; [
              cargo-llvm-cov
              cargo-nextest
              just
              mold-wrapped
              rustPlatform.bindgenHook
              rustToolchain
            ];

            RUSTFLAGS = "-C linker=clang -C link-arg=-fuse-ld=mold";
          };
        }
      );
    };
}
