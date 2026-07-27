{ pkgs, perSystem, inputs, ... }:

pkgs.dockerTools.buildLayeredImage {
  name = "quackster";
  tag = "latest";

  contents = [
    perSystem.self.default
    pkgs.dockerTools.caCertificates
  ];

  config = {
    Cmd = [ "/bin/quackster" ];
    # "::" is dual-stack on Linux (v4-mapped); v4-only bind gets RST when
    # pasta forwards host ::1 connections as IPv6
    Env = [ "APP_HOST=::" ];
    ExposedPorts."3000/tcp" = { };
    Labels = {
      # ghcr auto-links the package to the repo via .source
      "org.opencontainers.image.source" = "https://github.com/quacksterparty/quackster";
      "org.opencontainers.image.revision" = inputs.self.rev or "dirty";
      "org.opencontainers.image.description" = "Open-source self hostable quiz platform";
      "org.opencontainers.image.licenses" = "EUPL-1.2";
    };
  };
}
