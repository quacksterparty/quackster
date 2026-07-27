{ pkgs, inputs, ... }:

let
  craneLib = inputs.crane.mkLib pkgs;

  commonArgs = {
    src = craneLib.cleanCargoSource ../../api;
    strictDeps = true;
  };
in
craneLib.buildPackage (
  commonArgs
  // {
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    doCheck = false;
  }
)
