#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
VERSION="$(sed -nE "s/^version[[:space:]]*=[[:space:]]*'([^']+)'.*/\1/p" "$ROOT/build.gradle" | head -n 1)"
PACKAGE_DIR="$ROOT/dist/linop-db-jdbc-plugin-$VERSION"
ZIP_PATH="$ROOT/dist/linop-db-jdbc-plugin-$VERSION.zip"
LATEST_ZIP_PATH="$ROOT/dist/linop-db-jdbc-plugin-latest.zip"

cd "$ROOT"
./gradlew -q shadowJar -x test

rm -rf "$PACKAGE_DIR" "$ZIP_PATH" "$LATEST_ZIP_PATH"
mkdir -p "$PACKAGE_DIR/bin" "$PACKAGE_DIR/lib"
cp "$ROOT/manifest.json" "$PACKAGE_DIR/manifest.json"
cp "$ROOT/bin/linop-db-jdbc-plugin" "$PACKAGE_DIR/bin/linop-db-jdbc-plugin"
cp "$ROOT/bin/linop-db-jdbc-plugin.bat" "$PACKAGE_DIR/bin/linop-db-jdbc-plugin.bat"
cp "$ROOT/bin/linop-db-maven-resolver" "$PACKAGE_DIR/bin/linop-db-maven-resolver"
cp "$ROOT/bin/linop-db-maven-resolver.bat" "$PACKAGE_DIR/bin/linop-db-maven-resolver.bat"
cp "$ROOT/build/libs/linop-db-jdbc-plugin-all.jar" "$PACKAGE_DIR/lib/linop-db-jdbc-plugin.jar"
chmod +x "$PACKAGE_DIR/bin/linop-db-jdbc-plugin"
chmod +x "$PACKAGE_DIR/bin/linop-db-maven-resolver"

(cd "$ROOT/dist" && zip -qr "linop-db-jdbc-plugin-$VERSION.zip" "linop-db-jdbc-plugin-$VERSION")
cp "$ZIP_PATH" "$LATEST_ZIP_PATH"
echo "$ZIP_PATH"
echo "$LATEST_ZIP_PATH"
