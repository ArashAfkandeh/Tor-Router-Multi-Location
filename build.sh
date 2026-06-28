#!/usr/bin/env bash
export DEBIAN_FRONTEND=noninteractive
# =============================================================================
#  build.sh — Tor Router — Full Build Script
#
#  Expected Directory Structure:
#
#  tor-router/
#  ├── build.sh
#  ├── assets/                ← Embedded files (include_bytes!)
#  │   ├── tor-bin
#  │   ├── geoip
#  │   └── geoip6
#  ├── daemon/                ← Rust Source
#  │   ├── Cargo.toml
#  │   └── src/
#  │       ├── main.rs        (include_bytes!("../../assets/tor-bin") ✓)
#  │       ├── api.rs
#  │       ├── cli.rs
#  │       ├── config.rs
#  │       ├── daemon.rs
#  │       └── tor_process.rs
#  └── webpanel/              ← Web Panel (Frontend)
#      ├── app.js
#      ├── Countries.html
#      ├── index.html
#      ├── package.json
#      ├── postcss.config.js
#      ├── style.css
#      ├── tailwind.config.js
#      └── src/
#          └── input.css
#
#  Final Output in: ./dist/
#    dist/<binary>    ← daemon
#    dist/web/        ← web panel (if present)
#    dist/run.sh      ← quick run script
# =============================================================================

set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

# ─── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

# ─── Paths ───────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_DIR="$SCRIPT_DIR/daemon"
WEB_DIR="$SCRIPT_DIR/webpanel"
ASSETS_DIR="$SCRIPT_DIR/assets"
DIST_DIR="$SCRIPT_DIR/dist"

# ─── Defaults ────────────────────────────────────────────────────────────────
BUILD_MODE="release"
BUILD_DAEMON=true
BUILD_WEB=true
CLEAN_FIRST=false
TARGET=""
VERBOSE=false

# ─── Setup Sudo ──────────────────────────────────────────────────────────────
SUDO=""
if [ "$EUID" -ne 0 ] && command -v sudo ; then
    SUDO="sudo"
fi

# ─── Utilities ───────────────────────────────────────────────────────────────
log_info()    { echo -e "${CYAN}[INFO]${RESET}  $*"; }
log_ok()      { echo -e "${GREEN}[OK]${RESET}    $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${RESET}  $*"; }
log_error()   { echo -e "${RED}[ERROR]${RESET} $*" >&2; }
log_step()    { echo -e "\n${BOLD}${CYAN}▶ $*${RESET}"; }
log_section() {
    echo ""
    echo -e "${BOLD}${CYAN}╔════════════════════════════════════════════╗${RESET}"
    printf "${BOLD}${CYAN}║  %-42s ║${RESET}\n" "$*"
    echo -e "${BOLD}${CYAN}╚════════════════════════════════════════════╝${RESET}"
}

usage() {
    echo -e "${BOLD}Usage:${RESET} $0 [options]"
    echo ""
    echo "  --debug           Debug build (default: release)"
    echo "  --release         Release build"
    echo "  --clean           Clean before build"
    echo "  --daemon-only     Build Rust daemon only"
    echo "  --web-only        Build web panel only"
    echo "  --target <T>      Cross-compile (e.g., x86_64-unknown-linux-musl)"
    echo "  --verbose         Verbose cargo output"
    echo "  -h, --help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                                   # Full release build"
    echo "  $0 --debug                           # Debug build"
    echo "  $0 --clean --release                 # Clean + build"
    echo "  $0 --target x86_64-unknown-linux-musl"
    exit 0
}

# ─── Parse Arguments ─────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug)        BUILD_MODE="debug"  ;;
        --release)      BUILD_MODE="release" ;;
        --clean)        CLEAN_FIRST=true    ;;
        --daemon-only)  BUILD_WEB=false     ;;
        --web-only)     BUILD_DAEMON=false  ;;
        --verbose)      VERBOSE=true        ;;
        --target)       TARGET="$2"; shift  ;;
        -h|--help)      usage               ;;
        *) log_error "Unknown argument: $1"; usage ;;
    esac
    shift
done

# ─── Install rsync if missing ───────────────────────────────────────────────
install_rsync() {
    log_warn "rsync not found. Installing rsync..."
    echo ""
    
    # Detect package manager
    if command -v apt ; then
        log_info "Detected apt package manager (Debian/Ubuntu)"
        $SUDO apt update  || true
        $SUDO apt install -y rsync 
    elif command -v yum ; then
        log_info "Detected yum package manager (CentOS/RHEL)"
        $SUDO yum install -y rsync 
    elif command -v dnf ; then
        log_info "Detected dnf package manager (Fedora)"
        $SUDO dnf install -y rsync 
    elif command -v apk ; then
        log_info "Detected apk package manager (Alpine)"
        $SUDO apk add rsync 
    elif command -v pacman ; then
        log_info "Detected pacman package manager (Arch)"
        $SUDO pacman -S rsync 
    else
        log_error "Could not detect package manager. Please install rsync manually."
        exit 1
    fi
    
    # Verify installation
    if ! command -v rsync ; then
        log_error "Failed to install rsync. Please install manually."
        exit 1
    fi
    
    log_ok "rsync installed successfully: $(rsync --version | head -n1)"
    echo ""
}

# ─── Install build-essential if missing ─────────────────────────────────────
install_build_essential() {
    log_warn "C compiler/linker (cc) not found. Installing build-essential..."
    echo ""
    
    # Detect package manager
    if command -v apt ; then
        log_info "Detected apt package manager (Debian/Ubuntu)"
        $SUDO apt update  || true
        $SUDO apt install -y build-essential 
    elif command -v yum ; then
        log_info "Detected yum package manager (CentOS/RHEL)"
        $SUDO yum groupinstall -y "Development Tools" 
    elif command -v dnf ; then
        log_info "Detected dnf package manager (Fedora)"
        $SUDO dnf groupinstall -y "Development Tools" 
    elif command -v apk ; then
        log_info "Detected apk package manager (Alpine)"
        $SUDO apk add build-base 
    else
        log_error "Could not detect package manager. Please install build-essential manually:"
        echo "  Debian/Ubuntu: apt install -y -qq build-essential"
        echo "  CentOS/RHEL:   yum groupinstall -y 'Development Tools'"
        echo "  Fedora:        dnf groupinstall -y 'Development Tools'"
        echo "  Alpine:        apk add build-base"
        exit 1
    fi
    
    # Verify installation
    if ! command -v cc ; then
        log_error "Failed to install C compiler. Please install manually."
        exit 1
    fi
    
    log_ok "C compiler installed successfully: $(cc --version | head -n1)"
    echo ""
}

# ─── Install Rust if missing ────────────────────────────────────────────────
install_rust() {
    # Check if Rust is already installed
    if command -v cargo ; then
        log_info "Rust/Cargo already installed: $(cargo --version)"
        return 0
    fi
    
    log_warn "Rust/Cargo not found. Installing Rust..."
    echo ""
    
    # Check if rustup is already installed but not in PATH
    if command -v rustup ; then
        log_info "rustup found, installing Rust toolchain..."
        rustup default stable 
        rustup update 
    else
        # Install rustup with quiet mode
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 
        
        # Source cargo environment for current session
        source "$HOME/.cargo/env" 
    fi
    
    # Verify installation
    if ! command -v cargo ; then
        log_error "Failed to install Rust/Cargo. Please install manually:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    log_ok "Rust/Cargo installed successfully: $(cargo --version)"
    echo ""
}

# ─── Install Node.js if missing ─────────────────────────────────────────────
install_nodejs() {
    log_warn "Node.js/npm not found. Installing Node.js..."
    echo ""
    
    # Detect package manager and install Node.js
    if command -v apt ; then
        log_info "Detected apt package manager (Debian/Ubuntu)"
        # Add NodeSource repository for latest Node.js
        curl -fsSL https://deb.nodesource.com/setup_20.x | $SUDO bash - 
        $SUDO apt install -y nodejs 
    elif command -v yum ; then
        log_info "Detected yum package manager (CentOS/RHEL)"
        curl -fsSL https://rpm.nodesource.com/setup_20.x | $SUDO bash - 
        $SUDO yum install -y nodejs 
    elif command -v dnf ; then
        log_info "Detected dnf package manager (Fedora)"
        curl -fsSL https://rpm.nodesource.com/setup_20.x | $SUDO bash - 
        $SUDO dnf install -y nodejs 
    elif command -v apk ; then
        log_info "Detected apk package manager (Alpine)"
        $SUDO apk add nodejs npm 
    else
        log_error "Could not detect package manager. Please install Node.js manually:"
        echo "  Visit: https://nodejs.org/"
        exit 1
    fi
    
    # Verify installation
    if ! command -v node  || ! command -v npm ; then
        log_error "Failed to install Node.js/npm. Please install manually."
        echo "  Visit: https://nodejs.org/"
        exit 1
    fi
    
    log_ok "Node.js installed successfully: $(node --version)"
    log_ok "npm installed successfully: $(npm --version)"
    echo ""
}

# ─── Install Tor dependencies (libevent) ──────────────────────────────────
install_tor_deps() {
    log_info "Installing dependencies for Tor binary (libevent, libssl)..."
    if command -v apt ; then
        $SUDO apt update  || true
        $SUDO apt install -y tor || true 
    elif command -v yum ; then
        $SUDO yum install -y epel-release 
        $SUDO yum install -y tor 
    elif command -v dnf ; then
        $SUDO dnf install -y tor 
    elif command -v apk ; then
        $SUDO apk add tor 
    elif command -v pacman ; then
        $SUDO pacman -S tor 
    else
        log_warn "Could not detect package manager to install Tor dependencies."
    fi
}

# ─── Check and install tools ────────────────────────────────────────────────
check_tool() {
    if ! command -v "$1" ; then
        case "$1" in
            cargo|rustc)
                install_rust
                ;;
            cc|gcc)
                install_build_essential
                ;;
            npm|node|nodejs)
                install_nodejs
                ;;
            rsync)
                install_rsync
                ;;
            tor)
                install_tor_deps
                ;;
            *)
                log_error "Tool '$1' not found.${2:+  Hint: $2}"
                exit 1
                ;;
        esac
    fi
}

# ─── Start ───────────────────────────────────────────────────────────────────
log_section "Tor Router — Build System"
log_info "Mode : ${BOLD}$BUILD_MODE${RESET}  |  Root : $SCRIPT_DIR"
[[ -n "$TARGET" ]] && log_info "Target : $TARGET"

# ─── Cleanup ─────────────────────────────────────────────────────────────────
if $CLEAN_FIRST; then
    log_step "Cleaning..."
    rm -rf "$DIST_DIR"
    [[ -d "$DAEMON_DIR" ]] && (cd "$DAEMON_DIR" && cargo clean )
    rm -rf "$WEB_DIR/dist" "$WEB_DIR/.vite" "$WEB_DIR/.next" "$WEB_DIR/build"  || true
    log_ok "Cleaned."
fi
mkdir -p "$DIST_DIR"

# ══════════════════════════════════════════════════════════════════════════════
#  Phase 1 — Build Daemon (Rust)
# ══════════════════════════════════════════════════════════════════════════════
if $BUILD_DAEMON; then
    log_section "Phase 1 — Daemon (Rust)"
    
    # Check for C compiler (will auto-install if missing)
    check_tool cc
    
    # Check for cargo (will auto-install if missing)
    check_tool cargo
    
    # Check for tor (will auto-install if missing, to satisfy libevent)
    check_tool tor
    
    # Make sure cargo is in PATH for this session
    export PATH="$HOME/.cargo/bin:$PATH"

    [[ ! -d "$DAEMON_DIR" ]]        && log_error "Daemon directory not found: $DAEMON_DIR"     && exit 1
    [[ ! -f "$DAEMON_DIR/Cargo.toml" ]] && log_error "Cargo.toml not found."              && exit 1

    # ─── Check Assets ────────────────────────────────────────────────────────
    log_step "Checking assets..."
    MISSING=()
    [[ ! -f "$ASSETS_DIR/tor-bin" ]] && MISSING+=("assets/tor-bin")
    [[ ! -f "$ASSETS_DIR/geoip"   ]] && MISSING+=("assets/geoip")
    [[ ! -f "$ASSETS_DIR/geoip6"  ]] && MISSING+=("assets/geoip6")

    if [[ ${#MISSING[@]} -gt 0 ]]; then
        log_error "The following files are required for compilation and are missing:"
        for f in "${MISSING[@]}"; do echo -e "   ${RED}✗${RESET}  $SCRIPT_DIR/$f"; done
        echo ""
        echo "  Solution: Place the files tor-bin, geoip, geoip6 into the assets/ folder."
        echo "  (Compile path: daemon/src/../../assets/  →  assets/)"
        exit 1
    fi
    log_ok "All assets present."

    # ─── Cargo build ─────────────────────────────────────────────────────────
    log_step "Compiling Rust (${BUILD_MODE})..."
    CARGO_ARGS=("build")
    [[ "$BUILD_MODE" == "release" ]] && CARGO_ARGS+=("--release")
    [[ -n "$TARGET" ]]               && CARGO_ARGS+=("--target" "$TARGET")
    $VERBOSE                         && CARGO_ARGS+=("--verbose")
    # Removed quiet flag logic

    if [[ -n "$TARGET" ]] && ! rustup target list --installed  | grep "$TARGET"; then
        log_warn "Target '$TARGET' is not installed — installing..."
        rustup target add "$TARGET" 
    fi

    T0=$(date +%s)
    (cd "$DAEMON_DIR" && cargo "${CARGO_ARGS[@]}")
    log_ok "Compiled in $(($(date +%s) - T0))s."

    # ─── Copy Binary ─────────────────────────────────────────────────────────
    BIN_NAME=$(grep -m1 '^name' "$DAEMON_DIR/Cargo.toml" | sed 's/.*= *"\(.*\)"/\1/')
    BIN_NAME="${BIN_NAME:-tor-router}"
    [[ "$TARGET" == *"windows"* ]] && BIN_NAME="${BIN_NAME}.exe"

    OUT_NAME="ToRouter"
    [[ "$TARGET" == *"windows"* ]] && OUT_NAME="${OUT_NAME}.exe"

    if [[ -n "$TARGET" ]]; then
        CARGO_OUT="$DAEMON_DIR/target/$TARGET/$BUILD_MODE"
    else
        CARGO_OUT="$DAEMON_DIR/target/$BUILD_MODE"
    fi

    [[ ! -f "$CARGO_OUT/$BIN_NAME" ]] && log_error "Binary not found: $CARGO_OUT/$BIN_NAME" && exit 1

    cp "$CARGO_OUT/$BIN_NAME" "$DIST_DIR/$OUT_NAME"
    chmod +x "$DIST_DIR/$OUT_NAME"
    
    # Strip binary to reduce size
    if command -v strip ; then
        strip "$DIST_DIR/$OUT_NAME"
    fi
    
    log_ok "→ dist/$OUT_NAME  ($(du -sh "$DIST_DIR/$OUT_NAME" | cut -f1))"
fi

# ══════════════════════════════════════════════════════════════════════════════
#  Phase 2 — Build Web Panel
# ══════════════════════════════════════════════════════════════════════════════
if $BUILD_WEB; then
    if [[ ! -d "$WEB_DIR" || ! -f "$WEB_DIR/index.html" ]]; then
        log_info "webpanel/ directory or index.html not found — skipped."
    else
        log_section "Phase 2 — Web Panel"

        log_step "Building Tailwind CSS..."
        check_tool npm
        
        if [[ -f "$WEB_DIR/package.json" ]]; then
            # Install dependencies quietly
            (cd "$WEB_DIR" && { npm ci  || npm install ; })
            
            # Update caniuse-lite to remove warning
            (cd "$WEB_DIR" && npx --yes update-browserslist-db@latest  || true)
            
            # Build CSS quietly
            (cd "$WEB_DIR" && npm run build:css )
            log_ok "Tailwind CSS compiled."
        else
            log_warn "webpanel/package.json not found — skipping CSS build."
        fi

        log_step "Copying web panel files..."
        check_tool rsync
        rm -rf "$DIST_DIR/web"
        mkdir -p "$DIST_DIR/web"
        rsync -a \
            --exclude 'node_modules' \
            --exclude 'src' \
            --exclude 'package.json' \
            --exclude 'package-lock.json' \
            --exclude 'tailwind.config.js' \
            --exclude 'postcss.config.js' \
            "$WEB_DIR/" "$DIST_DIR/web/" 
        log_ok "→ dist/web/"
    fi
fi

# ══════════════════════════════════════════════════════════════════════════════
#  Phase 3 — Helper Files
# ══════════════════════════════════════════════════════════════════════════════
log_section "Phase 3 — Helper Files"

BIN_FINAL="ToRouter"
[[ "$TARGET" == *"windows"* ]] && BIN_FINAL="${BIN_FINAL}.exe"

# ─── run.sh ──────────────────────────────────────────────────────────────────
cat > "$DIST_DIR/run.sh" << RUNEOF
#!/usr/bin/env bash
export DEBIAN_FRONTEND=noninteractive
# Run Tor Router
DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
BIN="\$DIR/${BIN_FINAL}"

if [[ -d "\$DIR/web" ]]; then
    exec "\$BIN" --web-dir "\$DIR/web"
else
    exec "\$BIN" --run
fi
RUNEOF
chmod +x "$DIST_DIR/run.sh"

[[ -f "$SCRIPT_DIR/README.md" ]] && cp "$SCRIPT_DIR/README.md" "$DIST_DIR/"

# ─── Summary ─────────────────────────────────────────────────────────────────
log_section "Build Result"
echo -e "${GREEN}✅ Build successful!${RESET}\n"
echo -e "📦 Contents of dist/:"
ls -lh "$DIST_DIR"
echo ""
echo -e "${BOLD}${CYAN}Run:${RESET}"
echo -e "  ${CYAN}cd $DIST_DIR && ./run.sh${RESET}"
echo ""
echo -e "${BOLD}${CYAN}Or directly:${RESET}"
echo -e "  ${CYAN}$DIST_DIR/$BIN_FINAL --run${RESET}"
echo -e "  ${CYAN}$DIST_DIR/$BIN_FINAL --web-dir ./web${RESET}"
echo ""
