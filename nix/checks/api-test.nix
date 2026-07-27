{ pkgs, inputs, ... }:

let
  inherit (pkgs) lib;
  craneLib = inputs.crane.mkLib pkgs;

  commonArgs = {
    src = craneLib.cleanCargoSource ../../api;
    strictDeps = true;
  };
in
craneLib.cargoTest {
  pname = "api";
  version = (lib.importTOML ../../api/Cargo.toml).package.version;
  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
  cargoVendorDir = craneLib.vendorCargoDeps commonArgs;

  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ../../api/Cargo.toml
      ../../api/Cargo.lock
      ../../api/src
      ../../data
    ];
  };
  sourceRoot = "source/api";
}
