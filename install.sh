#!/bin/bash

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
APP_DIR="/opt/ToRouter-Multi-Location"
INSTALLATION_SCRIPT="${APP_DIR}/installation.sh"
REPO_OWNER="ArashAfkandeh"
REPO_NAME="ToRouter-Multi-Location"
GITHUB_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"
TARBALL_PATH="/root/ToRouter-Multi-Location.tar.gz"

# Function to print colored output
print_colored() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to print header
print_header() {
    clear
    print_colored "$CYAN" "╔═══════════════════════════════════════════════════════════════╗"
    print_colored "$CYAN" "║          📦 ToRouter Installation Manager v2.3               ║"
    print_colored "$CYAN" "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
}

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_colored "$RED" "✗ Error: This script must be run as root (use sudo)"
        exit 1
    fi
}

# Get latest version tag
get_latest_version() {
    local tag=$(curl -s "${GITHUB_API}/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' | tr -d '[:space:]')
    echo "$tag"
}

# Get actual asset filename for download
get_asset_name() {
    local tag=$1
    local asset=$(curl -s "${GITHUB_API}/tags/${tag}" | grep -o '"name": "[^"]*tar.gz"' | head -1 | sed -E 's/.*"name": "([^"]+)".*/\1/')
    if [ -z "$asset" ]; then
        # Fallback
        asset="ToRouter-Multi-Location-v0.1.0.tar.gz"
    fi
    echo "$asset"
}

# Function to download the tarball
download_tarball() {
    local requested_version=$1
    local tag
    local asset_name
    local download_url
    
    if [ -z "$requested_version" ] || [ "$requested_version" = "latest" ]; then
        print_colored "$YELLOW" "🔍 Fetching latest release..."
        tag=$(get_latest_version)
        print_colored "$GREEN" "✓ Latest tag detected: ${tag}"
    else
        tag="$requested_version"
        print_colored "$YELLOW" "🔍 Using specified version/tag: ${tag}"
    fi
    
    asset_name=$(get_asset_name "$tag")
    download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${tag}/${asset_name}"
    
    print_colored "$YELLOW" "📥 Downloading: ${asset_name}"
    curl -L -o "$TARBALL_PATH" "$download_url" --fail --silent --show-error
    
    if [ $? -ne 0 ] || [ ! -f "$TARBALL_PATH" ]; then
        print_colored "$RED" "✗ Error: Failed to download from ${download_url}"
        exit 1
    fi
    
    print_colored "$GREEN" "✓ Download completed successfully"
}

# Function to extract the tarball
extract_tarball() {
    print_colored "$YELLOW" "📂 Extracting package to /opt..."
    tar -xzf "$TARBALL_PATH" -C /opt
    if [ $? -ne 0 ]; then
        print_colored "$RED" "✗ Error: Failed to extract the package"
        exit 1
    fi
    print_colored "$GREEN" "✓ Package extracted successfully to $APP_DIR"
}

# Function to clean up the tarball
cleanup_tarball() {
    print_colored "$YELLOW" "🗑 Removing downloaded tarball..."
    rm -f "$TARBALL_PATH"
    print_colored "$GREEN" "✓ Tarball removed"
}

# Function to install dependencies
install_dependencies() {
    print_colored "$YELLOW" "📦 Installing dependencies..."
    apt update -qq
    apt install -y curl
    print_colored "$GREEN" "✓ Dependencies installed successfully"
}

# Function to execute installation script
execute_installation_script() {
    local command=$1
    
    if [ ! -f "$INSTALLATION_SCRIPT" ]; then
        print_colored "$RED" "✗ Error: Installation script not found at $INSTALLATION_SCRIPT"
        exit 1
    fi
    
    chmod +x "$INSTALLATION_SCRIPT"
    print_colored "$GREEN" "✓ Installation script is now executable"
    
    echo ""
    print_colored "$CYAN" "════════════════════════════════════════════════════════════════"
    print_colored "$GREEN" "▶ Executing: ${YELLOW}$command${GREEN}"
    print_colored "$CYAN" "════════════════════════════════════════════════════════════════"
    echo ""
    
    "$INSTALLATION_SCRIPT" "$command"
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        print_colored "$GREEN" "\n✅ Installation script executed successfully!"
    else
        print_colored "$RED" "\n❌ Installation script failed with exit code: $exit_code"
        exit $exit_code
    fi
}

# Full installation
full_install() {
    local version=$1
    print_header
    
    if [ -n "$version" ] && [ "$version" != "latest" ]; then
        print_colored "$GREEN" "🚀 Starting installation of ToRouter (version ${version})..."
    else
        print_colored "$GREEN" "🚀 Starting full installation of ToRouter (latest version)..."
    fi
    echo ""
    
    check_root
    install_dependencies
    echo ""
    download_tarball "$version"
    echo ""
    extract_tarball
    echo ""
    cleanup_tarball
    echo ""
    execute_installation_script "start"
}

# Show usage
show_usage() {
    print_header
    echo -e "${GREEN}Usage:${NC} $0 [${YELLOW}start${NC}|${YELLOW}stop${NC}|${YELLOW}uninstall${NC}|${YELLOW}VERSION${NC}]"
    echo ""
    echo -e "  ${GREEN}(no args)${NC}  → Latest version"
    echo -e "  ${GREEN}VERSION${NC}    → Specific tag (e.g. ToRouter)"
    echo ""
}

# ===================== MAIN =====================
check_root

case "$1" in
    start|stop|uninstall)
        execute_installation_script "$1"
        ;;
    "")
        full_install "latest"
        ;;
    *)
        full_install "$1"
        ;;
esac

exit 0
