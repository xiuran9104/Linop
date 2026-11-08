#!/bin/bash
set -euo pipefail

PRODUCT="${NAVOP_PRODUCT:-${ONETCLI_PRODUCT:-${2:-linop}}}"
case "$PRODUCT" in
    linop)
        APP_NAME="Linop"
        BINARY_NAME="linop"
        BUNDLE_ID="com.onetcli.app"
        PACKAGE_NAME="linop"
        ICNS_NAME="Linop"
        INCLUDE_DOCUMENT_TYPES=1
        ;;
    linop-shell)
        APP_NAME="Linop Shell"
        BINARY_NAME="linop-shell"
        BUNDLE_ID="com.linop.shell"
        PACKAGE_NAME="linop-shell"
        ICNS_NAME="LinopShell"
        INCLUDE_DOCUMENT_TYPES=0
        ;;
    linop-data)
        APP_NAME="Linop Data"
        BINARY_NAME="linop-data"
        BUNDLE_ID="com.linop.data"
        PACKAGE_NAME="linop-data"
        ICNS_NAME="LinopData"
        INCLUDE_DOCUMENT_TYPES=0
        ;;
    *)
        echo "Error: unknown product '${PRODUCT}' (expected linop, linop-shell, linop-data)"
        exit 1
        ;;
esac
TARGET="${1:-aarch64-apple-darwin}"
VERSION="${ONETCLI_VERSION:-0.1.0}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOURCE_PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_DIR="${ONETCLI_PROJECT_DIR:-$SOURCE_PROJECT_DIR}"
RESOURCE_DIR="${ONETCLI_MACOS_RESOURCE_DIR:-${PROJECT_DIR}/resources/macos}"
APP_DIR="${PROJECT_DIR}/target/${APP_NAME}.app"

echo "Bundling ${APP_NAME}.app for ${TARGET} (version: ${VERSION})..."

# Clean previous bundle
rm -rf "$APP_DIR"

# Create .app directory structure
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
BINARY_PATH="${ONETCLI_BINARY_PATH:-${PROJECT_DIR}/target/${TARGET}/release/${BINARY_NAME}}"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at ${BINARY_PATH}"
    echo "Run: cargo build --release -p ${PACKAGE_NAME} --target ${TARGET}"
    exit 1
fi
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/${BINARY_NAME}"

# Copy Info.plist and substitute product identity. The full product keeps the
# historical bundle id so upgrades, keychain items and file associations are
# preserved; standalone products get their own identity.
if [ "$INCLUDE_DOCUMENT_TYPES" = "1" ]; then
    sed -e "s/\${ONETCLI_VERSION}/${VERSION}/g" \
        -e "s|com.onetcli.app|${BUNDLE_ID}|g" \
        -e "/<key>CFBundleName<\/key>/{n;s|<string>Linop</string>|<string>${APP_NAME}</string>|;}" \
        -e "/<key>CFBundleDisplayName<\/key>/{n;s|<string>Linop</string>|<string>${APP_NAME}</string>|;}" \
        -e "/<key>CFBundleExecutable<\/key>/{n;s|<string>linop</string>|<string>${BINARY_NAME}</string>|;}" \
        "${RESOURCE_DIR}/Info.plist" \
        > "$APP_DIR/Contents/Info.plist"
else
    sed -e "s/\${ONETCLI_VERSION}/${VERSION}/g" \
        -e "s|com.onetcli.app|${BUNDLE_ID}|g" \
        -e "/<key>CFBundleName<\/key>/{n;s|<string>Linop</string>|<string>${APP_NAME}</string>|;}" \
        -e "/<key>CFBundleDisplayName<\/key>/{n;s|<string>Linop</string>|<string>${APP_NAME}</string>|;}" \
        -e "/<key>CFBundleExecutable<\/key>/{n;s|<string>linop</string>|<string>${BINARY_NAME}</string>|;}" \
        -e "/<!-- NAVOP_DOCUMENT_TYPES_START -->/,/<!-- NAVOP_DOCUMENT_TYPES_END -->/d" \
        "${RESOURCE_DIR}/Info.plist" \
        > "$APP_DIR/Contents/Info.plist"
fi

# Copy icon（三个产品各有自己的 icns，Info.plist 的 CFBundleIconFile 都是 Linop，
# 所以统一拷成 Linop.icns）
ICNS_PATH="${RESOURCE_DIR}/${ICNS_NAME}.icns"
if [ ! -f "$ICNS_PATH" ]; then
    echo "Error: Icon file not found at ${ICNS_PATH}"
    exit 1
fi
cp "$ICNS_PATH" "$APP_DIR/Contents/Resources/Linop.icns"

# Write PkgInfo
echo -n "APPL????" > "$APP_DIR/Contents/PkgInfo"

# Sign the app bundle before packaging it. A real Developer ID identity is
# preferred; ad-hoc signing still binds Info.plist and the bundle identifier.
APP_SIGN_IDENTITY="${MACOS_APP_SIGN_IDENTITY:-${MACOS_SIGN_IDENTITY:--}}"
if [ "$APP_SIGN_IDENTITY" = "-" ]; then
    echo "Ad-hoc signing ${APP_NAME}.app..."
    codesign --force --deep --sign - "$APP_DIR"
else
    echo "Signing ${APP_NAME}.app with identity: ${APP_SIGN_IDENTITY}"
    codesign \
        --force \
        --deep \
        --options runtime \
        --timestamp \
        --sign "$APP_SIGN_IDENTITY" \
        "$APP_DIR"
fi
codesign --verify --deep --strict --verbose=2 "$APP_DIR"

echo "Successfully built: ${APP_DIR}"
echo "Contents:"
ls -la "$APP_DIR/Contents/"
ls -la "$APP_DIR/Contents/MacOS/"
ls -la "$APP_DIR/Contents/Resources/"
