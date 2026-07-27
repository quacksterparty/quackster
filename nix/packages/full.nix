{ pkgs, perSystem, ... }:

pkgs.symlinkJoin {
  name = "quackster-full";
  paths = [ perSystem.self.default ];
  nativeBuildInputs = [ pkgs.makeWrapper ];
  postBuild = ''
    wrapProgram $out/bin/quackster \
      --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.yt-dlp ]}
  '';
  inherit (perSystem.self.default) meta;
}
