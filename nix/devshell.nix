{ pkgs, ... }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    nodejs_26 # keep in sync with nix/packages/frontend.nix
    pnpm_11
    playwright-driver.browsers

    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer
    bacon

    tuxedo
    nix-update

    pkg-config
  ];

  PLAYWRIGHT_NODEJS_PATH = "${pkgs.nodejs_26}/bin/node";
  PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
  PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = 1;
  PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = true;
  PLAYWRIGHT_HOST_PLATFORM_OVERRIDE = "ubuntu-24.04";
}
