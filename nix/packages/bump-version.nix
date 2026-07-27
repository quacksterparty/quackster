{ pkgs, ... }:

pkgs.writeShellApplication {
  name = "bump-version";
  runtimeInputs = with pkgs; [
    cargo
    git
  ];
  text = ''
    usage() {
      echo "usage: bump-version <major|minor|patch|X.Y.Z>" >&2
      exit 1
    }

    cd "$(git rev-parse --show-toplevel)"

    current=$(sed -n 's/^version = "\(.*\)"/\1/p' api/Cargo.toml | head -1)
    IFS=. read -r major minor patch <<< "$current"

    case "''${1:-}" in
      major) new="$((major + 1)).0.0" ;;
      minor) new="$major.$((minor + 1)).0" ;;
      patch) new="$major.$minor.$((patch + 1))" ;;
      [0-9]*.[0-9]*.[0-9]*) new=$1 ;;
      *) usage ;;
    esac

    if ! git diff --quiet || ! git diff --cached --quiet; then
      echo "working tree not clean, commit or stash first" >&2
      exit 1
    fi

    sed -i "0,/^version = .*/s//version = \"$new\"/" api/Cargo.toml
    (cd api && cargo update -q --package api)

    git add api/Cargo.toml api/Cargo.lock
    git commit -q -m "release: v$new"
    git tag "v$new"

    echo "$current -> $new"
    echo "release with: git push && git push origin v$new"
  '';
}
