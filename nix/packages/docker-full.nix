{ pkgs, perSystem, inputs, ... }:

pkgs.dockerTools.buildLayeredImage {
  name = "quackster";
  tag = "full";

  contents = [
    perSystem.self.full
    pkgs.dockerTools.caCertificates
  ];

  # yt-dlp needs a writable cache and tmp; store cwd is read-only
  fakeRootCommands = ''
    mkdir -p tmp cache
    chmod 1777 tmp cache
  '';

  config = {
    Cmd = [ "/bin/quackster" ];
    User = "65534:65534"; # nobody; writable dirs are 1777, port 3000 is unprivileged
    Env = [
      "APP_HOST=::"
      "APP_YTDLP_ENABLED=true"
      "APP_MEDIA_CACHE_DIR=/cache/yt"
    ];
    ExposedPorts."3000/tcp" = { };
    Volumes."/cache" = { };
    Labels = {
      # ghcr auto-links the package to the repo via .source
      "org.opencontainers.image.source" = "https://github.com/quacksterparty/quackster";
      "org.opencontainers.image.revision" = inputs.self.rev or "dirty";
      "org.opencontainers.image.description" = "Open-source self hostable quiz platform";
      "org.opencontainers.image.licenses" = "EUPL-1.2";
    };
  };
}
