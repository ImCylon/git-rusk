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

          # Runtime dependency: the tool shells out to git for init/branch/checkout ops.
          # Must be in buildInputs (NOT nativeBuildInputs) so it propagates into the
          # nix profile install closure on pristine systems (RESEARCH.md §Pitfall 4).
          buildInputs = [ pkgs.git ];

          # Build-time hook: provides the `installShellCompletion` shell function used
          # in postInstall to place completion scripts in the per-shell share/ dirs.
          nativeBuildInputs = [ pkgs.installShellFiles ];

          # Harvest completions from the freshly-built binary (DIST-02 ↔ DIST-03 coupling).
          # Process substitution <(...) runs the binary and pipes stdout into
          # installShellCompletion, which writes each script to the right per-shell
          # directory under $out/share/.
          postInstall = ''
            installShellCompletion --cmd git-rusk \
              --bash <($out/bin/git-rusk completions bash) \
              --zsh  <($out/bin/git-rusk completions zsh) \
              --fish <($out/bin/git-rusk completions fish)
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
