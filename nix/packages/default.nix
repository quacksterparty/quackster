{ pkgs, perSystem, ... }:

let
  inherit (pkgs) lib stdenv makeWrapper;
  pname = "quackster";
in
stdenv.mkDerivation {
  inherit pname;
  version = (builtins.fromTOML (builtins.readFile ../../api/Cargo.toml)).package.version;

  dontUnpack = true;

  nativeBuildInputs = [ makeWrapper ];

  # binary resolves ../build and ../data relative to its cwd,
  # hence the api/ subdir and the --chdir in the wrapper
  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin $out/share/${pname}/api
    ln -s ${perSystem.self.api}/bin/api $out/share/${pname}/api/api
    ln -s ${perSystem.self.frontend} $out/share/${pname}/build
    cp -r ${../../data} $out/share/${pname}/data

    makeWrapper $out/share/${pname}/api/api $out/bin/${pname} \
      --chdir $out/share/${pname}/api

    runHook postInstall
  '';

  meta = {
    description = "Open-source self hostable quiz platform";
    homepage = "https://github.com/quacksterparty/quackster";
    license = lib.licenses.eupl12;
    mainProgram = pname;
    platforms = lib.platforms.unix;
  };
}
