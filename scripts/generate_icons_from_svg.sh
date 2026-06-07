#!/usr/bin/env bash

# Exit immediately if any command in the pipeline fails
set -euo pipefail

# Print usage instructions
usage() {
    echo "Usage: $0 <path_to_svg>"
    echo "Example: $0 assets/logo.svg"
    exit 1
}

# Ensure exactly one argument is provided
if [ "$#" -ne 1 ]; then
    usage
fi

SVG_PATH="$1"

# Verify that the input file actually exists and is a regular file
if [ ! -f "$SVG_PATH" ]; then
    echo "Error: File '$SVG_PATH' does not exist or is not a regular file." >&2
    exit 1
fi

# Determine parent directory and the base name (without .svg extension)
PARENT_DIR=$(dirname "$SVG_PATH")
FILENAME=$(basename "$SVG_PATH")
BASE_NAME=$(echo "$FILENAME" | sed -E 's/\.svg$//I')

# Target outputs to keep
TARGET_ICO="${PARENT_DIR}/${BASE_NAME}.ico"
TARGET_1024="${PARENT_DIR}/icon_1024x.png"
TARGET_512="${PARENT_DIR}/icon_512x.png"
TARGET_280="${PARENT_DIR}/icon_280x.png"
TARGET_256="${PARENT_DIR}/icon_256x.png"
TARGET_128="${PARENT_DIR}/icon_128x.png"
TARGET_64="${PARENT_DIR}/icon_64x.png"
TARGET_48="${PARENT_DIR}/icon_48x.png"
TARGET_32="${PARENT_DIR}/icon_32x.png"
TARGET_16="${PARENT_DIR}/icon_16x.png"

# Strict overwrite checks
TARGETS=(
    "$TARGET_ICO"
    "$TARGET_1024"
    "$TARGET_512"
    "$TARGET_280"
    "$TARGET_256"
    "$TARGET_128"
    "$TARGET_64"
    "$TARGET_48"
    "$TARGET_32"
    "$TARGET_16"
)

for target in "${TARGETS[@]}"; do
    if [ -e "$target" ]; then
        echo "Error: Destination file '$target' already exists. Aborting to prevent overwrite." >&2
        exit 1
    fi
done

# Ensure all binary dependencies are present in the PATH
for cmd in rsvg-convert oxipng icotool; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: Required system tool '$cmd' is not installed. (Install via 'pacman -S librsvg oxipng icoutils')" >&2
        exit 1
    fi
done

echo "-> Render stages: Generating PNG sizes..."
rsvg-convert -w 16 -h 16 "$SVG_PATH" -o "$TARGET_16"
rsvg-convert -w 32 -h 32 "$SVG_PATH" -o "$TARGET_32"
rsvg-convert -w 48 -h 48 "$SVG_PATH" -o "$TARGET_48"
rsvg-convert -w 64 -h 64 "$SVG_PATH" -o "$TARGET_64"
rsvg-convert -w 128 -h 128 "$SVG_PATH" -o "$TARGET_128"
rsvg-convert -w 256 -h 256 "$SVG_PATH" -o "$TARGET_256"
rsvg-convert -w 280 -h 280 "$SVG_PATH" -o "$TARGET_280"
rsvg-convert -w 512 -h 512 "$SVG_PATH" -o "$TARGET_512"
rsvg-convert -w 1024 -h 1024 "$SVG_PATH" -o "$TARGET_1024"

echo "-> Purification: Stripping optional PNG metadata/chunks..."
oxipng --strip all -o 4 \
    "$TARGET_16" \
    "$TARGET_32" \
    "$TARGET_48" \
    "$TARGET_64" \
    "$TARGET_128" \
    "$TARGET_256" \
    "$TARGET_280" \
    "$TARGET_512" \
    "$TARGET_1024"

echo "-> Packaging: Compiling standard Windows ICO..."
# Compiles the ICO using standard sizes: 16, 32, 48, and 256
icotool -c -o "$TARGET_ICO" "$TARGET_16" "$TARGET_32" "$TARGET_48" "$TARGET_256"

echo "Finished! Output files created in '$PARENT_DIR':"
echo "  - icon_16x.png"
echo "  - icon_32x.png"
echo "  - icon_48x.png"
echo "  - icon_64x.png"
echo "  - icon_128x.png"
echo "  - icon_256x.png"
echo "  - icon_280x.png"
echo "  - icon_512x.png"
echo "  - icon_1024x.png"
echo "  - $(basename "$TARGET_ICO")"