{ pkgs, ... }:

let
  inherit (pkgs)
    lib
    stdenv
    nodejs_24
    pnpm_11
    pnpmConfigHook
    fetchPnpmDeps
    ;
in
stdenv.mkDerivation (finalAttrs: {
  pname = "quackster-frontend";
  version = (builtins.fromTOML (builtins.readFile ../../api/Cargo.toml)).package.version;

  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ../../package.json
      ../../pnpm-lock.yaml
      ../../pnpm-workspace.yaml
      ../../.npmrc
      ../../svelte.config.ts
      ../../vite.config.ts
      ../../tsconfig.json
      ../../src
      ../../static
      ../../messages
      ../../project.inlang
    ];
  };

  nativeBuildInputs = [
    nodejs_24
    pnpm_11
    pnpmConfigHook
  ];

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs) pname version src;
    pnpm = pnpm_11;
    fetcherVersion = 4;
    hash = "sha256-whUrhMGxI26cZNSgBk/2yUrDZhm9AFdDWOnzwRK1A04=";
  };

  buildPhase = ''
    runHook preBuild
    pnpm build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    cp -r build $out
    runHook postInstall
  '';
})
