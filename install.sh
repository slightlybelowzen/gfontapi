#!/usr/bin/env bash
set -e

# Colors for pretty output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Installing gfontapi...${NC}"

# Create installation directory
INSTALL_DIR="$HOME/.gfontapi"
BIN_DIR="$INSTALL_DIR/bin"
mkdir -p "$BIN_DIR"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Please install Rust first:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if git is installed
if ! command -v git &> /dev/null; then
    echo "Git is not installed. Please install Git first."
    exit 1
fi

# Clone or update the repository
REPO_DIR="$INSTALL_DIR/repo"
if [ -d "$REPO_DIR" ]; then
    echo -e "${BLUE}Updating repository...${NC}"
    cd "$REPO_DIR"
    git pull
else
    echo -e "${BLUE}Cloning repository...${NC}"
    git clone https://github.com/yourusername/your-repo.git "$REPO_DIR"
    cd "$REPO_DIR"
fi

# Build your Rust CLI
echo -e "${BLUE}Building Google Fonts CLI...${NC}"
cargo build --release
cp target/release/your-cli-name "$BIN_DIR/"

# Clone and build woff2
echo -e "${BLUE}Building woff2 compression tool...${NC}"
WOFF2_DIR="$INSTALL_DIR/woff2"
if [ ! -d "$WOFF2_DIR" ]; then
    git clone https://github.com/google/woff2.git "$WOFF2_DIR"
fi
cd "$WOFF2_DIR"
git pull

# Check for cmake
if ! command -v cmake &> /dev/null; then
    echo "CMake is not installed. Please install CMake first."
    exit 1
fi

# Build woff2
mkdir -p build
cd build
cmake ..
make
cp woff2_compress "$BIN_DIR/"

# Add to PATH if not already there
RC_FILE="$HOME/.$(basename $SHELL)rc"
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo -e "${BLUE}Adding to PATH in $RC_FILE${NC}"
    echo "" >> "$RC_FILE"
    echo "# Google Fonts CLI" >> "$RC_FILE"
    echo "export PATH=\"\$PATH:$BIN_DIR\"" >> "$RC_FILE"
    echo -e "${GREEN}Added to PATH. Please restart your terminal or run 'source $RC_FILE'${NC}"
else
    echo -e "${GREEN}Already in PATH${NC}"
fi

echo -e "${GREEN}Installation complete!${NC}"
echo -e "You can now use the Google Fonts CLI by running 'gfontapi'"