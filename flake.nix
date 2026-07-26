{
  description = "git-rusk: Git hook manager with branch protection + TOTP";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = self;

        commonArgs = {
          inherit src;
          strictDeps = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        package = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          # Tests run via `cargo test` in dev/CI. Disabled in the Nix build because
          # the pre-existing git-state-dependent integration tests (git_ops, hook_*,
          # commands::hook — see .planning/phases/07-distribution-packaging/deferred-items.md)
          # mutate CWD / git state and are non-deterministic inside the pure Nix sandbox.
          doCheck = false;

          # Runtime: git is required (the tool shells out for init/branch/checkout).
          # buildInputs alone does NOT put git on PATH in the binary's runtime
          # closure — wrapProgram injects it so `nix profile install` / `nix run`
          # work on pristine NixOS hosts without system-wide git.
          buildInputs = [ pkgs.git ];

          # installShellFiles → installShellCompletion; makeWrapper → wrapProgram.
          nativeBuildInputs = [ pkgs.installShellFiles pkgs.makeWrapper ];

          # 1) Harvest completions from the unwrapped binary (DIST-02 ↔ DIST-03).
          # 2) Wrap the binary so `git` is always on PATH at runtime.
          postInstall = ''
            installShellCompletion --cmd git-rusk \
              --bash <($out/bin/git-rusk completions bash) \
              --zsh  <($out/bin/git-rusk completions zsh) \
              --fish <($out/bin/git-rusk completions fish)
            wrapProgram $out/bin/git-rusk \
              --prefix PATH : ${lib.makeBinPath [ pkgs.git ]}
          '';

          meta = with pkgs.lib; {
            description = "Git hook manager with branch protection and TOTP verification";
            homepage = "https://github.com/git-rusk/git-rusk";
            license = licenses.mit;
            mainProgram = "git-rusk";
            platforms = platforms.all;
          };
        });
      in {
        packages.default = package;

        apps.default = flake-utils.lib.mkApp {
          drv = package;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [ rustToolchain ];
          shellHook = ''
            echo "✅ Rust dev environment ready!"
          '';
        };
      }
    );
}
