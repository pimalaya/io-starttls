{
  description = "I/O-free Rust coroutine to upgrade any plain stream to a secure one";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pimalaya = {
      url = "github:pimalaya/nix";
      flake = false;
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    (import inputs.pimalaya).mkFlakeOutputs inputs {
      shell = ./shell.nix;
    };
}
