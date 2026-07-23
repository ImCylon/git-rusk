{
  description = "git-rusk — Rust CLI para proteção de branches com TOTP";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        name = "git-rusk-dev";

        packages = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
          rust-analyzer
          git
        ];

        shellHook = ''
          echo ""
          echo "╔══════════════════════════════════════════╗"
          echo "║  git-rusk dev shell ativado              ║"
          echo "║  Rust: $(rustc --version)                 ║"
          echo "╚══════════════════════════════════════════╝"
          echo ""
        '';
      };
    };
}
