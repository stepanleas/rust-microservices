#!/usr/bin/env bash
# Downloads the kind version pinned in deploy/platform/versions.yaml to .tools/bin/kind.
. "$(dirname "$0")/lib.sh"

WANTED=$(version kind.version)
TARGET="$ROOT_DIR/.tools/bin/kind"

if [[ -x "$TARGET" ]] && "$TARGET" version | grep -q "kind $WANTED "; then
  echo "kind $WANTED already in $TARGET"
  exit 0
fi

mkdir -p "$(dirname "$TARGET")"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m | sed 's/x86_64/amd64/; s/aarch64/arm64/')
URL="https://github.com/kubernetes-sigs/kind/releases/download/$WANTED/kind-$OS-$ARCH"
echo "Downloading $URL"
curl -fsSL -o "$TARGET.tmp" "$URL"
curl -fsSL "$URL.sha256sum" | awk '{print $1}' > "$TARGET.sha256"
echo "$(cat "$TARGET.sha256")  $TARGET.tmp" | sha256sum --check --quiet
chmod +x "$TARGET.tmp"
mv "$TARGET.tmp" "$TARGET"
rm "$TARGET.sha256"
"$TARGET" version
