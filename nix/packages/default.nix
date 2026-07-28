{ pkgs, perSystem, ... }:
let
  inherit (pkgs) lib stdenv makeWrapper;
  pname = "quackster";
in
stdenv.mkDerivation {
  inherit pname;
  version = (lib.importTOML ../../api/Cargo.toml).package.version;

  dontUnpack = true;

  nativeBuildInputs = [ makeWrapper ];

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin $out/share/${pname}
    cp -r ${../../data} $out/share/${pname}/data
    ln -s ${perSystem.self.frontend} $out/share/${pname}/static

    makeWrapper ${perSystem.self.api}/bin/api $out/bin/${pname} \
      --set-default APP_STATIC_DIR $out/share/${pname}/static \
      --set-default APP_DATA_DIR $out/share/${pname}/data

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
