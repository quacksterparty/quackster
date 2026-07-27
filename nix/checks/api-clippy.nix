{ pkgs, inputs, ... }:

let
  craneLib = inputs.crane.mkLib pkgs;

  commonArgs = {
    src = craneLib.cleanCargoSource ../../api;
    strictDeps = true;
  };
in
craneLib.cargoClippy (
  commonArgs
  // {
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    cargoClippyExtraArgs = "--all-targets -- --deny warnings";
  }
)
