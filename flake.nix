{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "yawn";
          version = "0.10.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.installShellFiles ];
          nativeCheckInputs = [ pkgs.git ];
          postInstall = ''
            installShellCompletion --bash completions/yawn.bash
            installShellCompletion --zsh --name _yawn completions/yawn.zsh
            installShellCompletion --fish completions/yawn.fish
          '';
        };
      }
    );
}
