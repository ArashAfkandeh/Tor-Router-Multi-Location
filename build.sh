#!/usr/bin/env bash
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
#      ├── index.html
#      ├── app.js
#      ├── style.css
#      └── Countries.html
#
#  Final Output in: ./dist/
#    dist/<binary>    ← daemon
#    dist/web/        ← web panel (if present)
#    dist/run.sh      ← quick run script
# =============================================================================

set -euo pipefail

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

export PATH="$HOME/.cargo/bin:$PATH"

check_tool() {
    if ! command -v "$1" &>/dev/null; then
        log_error "Tool '$1' not found.${2:+  Hint: $2}"
        exit 1
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
    [[ -d "$DAEMON_DIR" ]] && (cd "$DAEMON_DIR" && cargo clean)
    rm -rf "$WEB_DIR/dist" "$WEB_DIR/.vite" "$WEB_DIR/.next" "$WEB_DIR/build" 2>/dev/null || true
    log_ok "Cleaned."
fi
mkdir -p "$DIST_DIR"

# ══════════════════════════════════════════════════════════════════════════════
#  Phase 1 — Build Daemon (Rust)
# ══════════════════════════════════════════════════════════════════════════════
if $BUILD_DAEMON; then
    log_section "Phase 1 — Daemon (Rust)"
    check_tool cargo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"

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

    if [[ -n "$TARGET" ]] && ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
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

        log_step "Copying web panel files..."
        rm -rf "$DIST_DIR/web"
        cp -r "$WEB_DIR" "$DIST_DIR/web"
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
