{ pkgs, perSystem, ... }:

let
  inherit (pkgs)
    lib
    stdenv
    nodejs_24
    pnpm_11
    pnpmConfigHook
    ;
in
stdenv.mkDerivation {
  pname = "quackster-frontend-check";
  inherit (perSystem.self.frontend) version pnpmDeps;

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
      ../../eslint.config.ts
      ../../.prettierrc
      ../../.prettierignore
      ../../.gitignore
      ../../playwright.config.ts
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

  buildPhase = ''
    runHook preBuild

    pnpm exec paraglide-js compile --project ./project.inlang --outdir ./src/lib/paraglide

    pnpm check
    pnpm lint
    pnpm exec vitest run --project server

    runHook postBuild
  '';

  installPhase = "touch $out";
}
