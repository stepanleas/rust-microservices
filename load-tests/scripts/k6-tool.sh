#!/usr/bin/env bash
# Downloads the k6 version pinned in deploy/platform/versions.yaml to .tools/bin/k6.
. "$(dirname "$0")/../../deploy/scripts/lib.sh"

WANTED=$(version k6.version)
TARGET="$ROOT_DIR/.tools/bin/k6"

if [[ -x "$TARGET" ]] && "$TARGET" version | grep -q "k6 $WANTED "; then
  echo "k6 $WANTED already in $TARGET"
  exit 0
fi

mkdir -p "$(dirname "$TARGET")"
ARCH=$(uname -m | sed 's/x86_64/amd64/; s/aarch64/arm64/')
NAME="k6-$WANTED-linux-$ARCH"
BASE="https://github.com/grafana/k6/releases/download/$WANTED"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
echo "Downloading $BASE/$NAME.tar.gz"
curl -fsSL -o "$TMP/$NAME.tar.gz" "$BASE/$NAME.tar.gz"
curl -fsSL "$BASE/k6-$WANTED-checksums.txt" | grep " $NAME.tar.gz\$" > "$TMP/checksum"
(cd "$TMP" && sha256sum --check --quiet checksum)
tar -xzf "$TMP/$NAME.tar.gz" -C "$TMP" "$NAME/k6"
mv "$TMP/$NAME/k6" "$TARGET"
"$TARGET" version
