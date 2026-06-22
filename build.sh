#!/usr/bin/env bash
# =============================================================================
#  build.sh — Tor Router — Full Build Script
#
#  ساختار پوشه‌ی مورد انتظار:
#
#  tor-router/
#  ├── build.sh
#  ├── assets/                ← فایل‌های embed (include_bytes!)
#  │   ├── tor-bin
#  │   ├── geoip
#  │   └── geoip6
#  ├── daemon/                ← سورس Rust
#  │   ├── Cargo.toml
#  │   └── src/
#  │       ├── main.rs        (include_bytes!("../../assets/tor-bin") ✓)
#  │       ├── api.rs
#  │       ├── cli.rs
#  │       ├── config.rs
#  │       ├── daemon.rs
#  │       └── tor_process.rs
#  └── web/                   ← وب پنل (آینده)
#      ├── package.json
#      └── src/
#
#  خروجی نهایی در: ./dist/
#    dist/<binary>    ← daemon
#    dist/web/        ← وب پنل (اگه باشه)
#    dist/run.sh      ← اجرای سریع
# =============================================================================

set -euo pipefail

# ─── رنگ‌ها ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

# ─── مسیرها ──────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_DIR="$SCRIPT_DIR/daemon"
WEB_DIR="$SCRIPT_DIR/web"
ASSETS_DIR="$SCRIPT_DIR/assets"
DIST_DIR="$SCRIPT_DIR/dist"

# ─── پیش‌فرض‌ها ───────────────────────────────────────────────────────────────
BUILD_MODE="release"
BUILD_DAEMON=true
BUILD_WEB=true
CLEAN_FIRST=false
TARGET=""
VERBOSE=false

# ─── ابزارهای کمکی ───────────────────────────────────────────────────────────
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
    echo -e "${BOLD}استفاده:${RESET} $0 [گزینه‌ها]"
    echo ""
    echo "  --debug           بیلد debug (پیش‌فرض: release)"
    echo "  --release         بیلد release"
    echo "  --clean           پاک‌سازی قبل از بیلد"
    echo "  --daemon-only     فقط daemon Rust"
    echo "  --web-only        فقط وب پنل"
    echo "  --target <T>      کراس‌کامپایل  (مثلاً x86_64-unknown-linux-musl)"
    echo "  --verbose         خروجی کامل cargo"
    echo "  -h, --help        این راهنما"
    echo ""
    echo "مثال‌ها:"
    echo "  $0                                   # بیلد کامل release"
    echo "  $0 --debug                           # بیلد debug"
    echo "  $0 --clean --release                 # پاک‌سازی + بیلد"
    echo "  $0 --target x86_64-unknown-linux-musl"
    exit 0
}

# ─── پارس آرگومان‌ها ──────────────────────────────────────────────────────────
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
        *) log_error "آرگومان ناشناخته: $1"; usage ;;
    esac
    shift
done

check_tool() {
    if ! command -v "$1" &>/dev/null; then
        log_error "ابزار '$1' پیدا نشد.${2:+  راهنمایی: $2}"
        exit 1
    fi
}

# ─── شروع ────────────────────────────────────────────────────────────────────
log_section "Tor Router — Build System"
log_info "حالت : ${BOLD}$BUILD_MODE${RESET}  |  ریشه : $SCRIPT_DIR"
[[ -n "$TARGET" ]] && log_info "Target : $TARGET"

# ─── پاک‌سازی ─────────────────────────────────────────────────────────────────
if $CLEAN_FIRST; then
    log_step "پاک‌سازی..."
    rm -rf "$DIST_DIR"
    [[ -d "$DAEMON_DIR" ]] && (cd "$DAEMON_DIR" && cargo clean)
    rm -rf "$WEB_DIR/dist" "$WEB_DIR/.vite" "$WEB_DIR/.next" "$WEB_DIR/build" 2>/dev/null || true
    log_ok "پاک شد."
fi
mkdir -p "$DIST_DIR"

# ══════════════════════════════════════════════════════════════════════════════
#  مرحله ۱ — بیلد Daemon (Rust)
# ══════════════════════════════════════════════════════════════════════════════
if $BUILD_DAEMON; then
    log_section "مرحله ۱ — Daemon (Rust)"
    check_tool cargo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"

    [[ ! -d "$DAEMON_DIR" ]]        && log_error "پوشه daemon پیدا نشد: $DAEMON_DIR"     && exit 1
    [[ ! -f "$DAEMON_DIR/Cargo.toml" ]] && log_error "Cargo.toml پیدا نشد."              && exit 1

    # ─── چک assets ────────────────────────────────────────────────────────────
    log_step "چک assets..."
    MISSING=()
    [[ ! -f "$ASSETS_DIR/tor-bin" ]] && MISSING+=("assets/tor-bin")
    [[ ! -f "$ASSETS_DIR/geoip"   ]] && MISSING+=("assets/geoip")
    [[ ! -f "$ASSETS_DIR/geoip6"  ]] && MISSING+=("assets/geoip6")

    if [[ ${#MISSING[@]} -gt 0 ]]; then
        log_error "فایل‌های زیر برای compile لازم هستند و وجود ندارند:"
        for f in "${MISSING[@]}"; do echo -e "   ${RED}✗${RESET}  $SCRIPT_DIR/$f"; done
        echo ""
        echo "  راه‌حل: فایل‌های tor-bin، geoip، geoip6 را در پوشه assets/ قرار دهید."
        echo "  (مسیر compile: daemon/src/../../assets/  →  assets/)"
        exit 1
    fi
    log_ok "همه assets موجودند."

    # ─── Cargo build ──────────────────────────────────────────────────────────
    log_step "کامپایل Rust (${BUILD_MODE})..."
    CARGO_ARGS=("build")
    [[ "$BUILD_MODE" == "release" ]] && CARGO_ARGS+=("--release")
    [[ -n "$TARGET" ]]               && CARGO_ARGS+=("--target" "$TARGET")
    $VERBOSE                         && CARGO_ARGS+=("--verbose")

    if [[ -n "$TARGET" ]] && ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
        log_warn "Target '$TARGET' نصب نیست — نصب می‌شود..."
        rustup target add "$TARGET"
    fi

    T0=$(date +%s)
    (cd "$DAEMON_DIR" && cargo "${CARGO_ARGS[@]}")
    log_ok "کامپایل در $(($(date +%s) - T0))s انجام شد."

    # ─── کپی باینری ───────────────────────────────────────────────────────────
    BIN_NAME=$(grep -m1 '^name' "$DAEMON_DIR/Cargo.toml" | sed 's/.*= *"\(.*\)"/\1/')
    BIN_NAME="${BIN_NAME:-tor-router}"
    [[ "$TARGET" == *"windows"* ]] && BIN_NAME="${BIN_NAME}.exe"

    if [[ -n "$TARGET" ]]; then
        CARGO_OUT="$DAEMON_DIR/target/$TARGET/$BUILD_MODE"
    else
        CARGO_OUT="$DAEMON_DIR/target/$BUILD_MODE"
    fi

    [[ ! -f "$CARGO_OUT/$BIN_NAME" ]] && log_error "باینری پیدا نشد: $CARGO_OUT/$BIN_NAME" && exit 1

    cp "$CARGO_OUT/$BIN_NAME" "$DIST_DIR/$BIN_NAME"
    chmod +x "$DIST_DIR/$BIN_NAME"
    log_ok "→ dist/$BIN_NAME  ($(du -sh "$DIST_DIR/$BIN_NAME" | cut -f1))"
fi

# ══════════════════════════════════════════════════════════════════════════════
#  مرحله ۲ — بیلد Web Panel
# ══════════════════════════════════════════════════════════════════════════════
if $BUILD_WEB; then
    if [[ ! -d "$WEB_DIR" || ! -f "$WEB_DIR/package.json" ]]; then
        log_info "پوشه web/ یا package.json پیدا نشد — رد شد."
    else
        log_section "مرحله ۲ — Web Panel"

        if   [[ -f "$WEB_DIR/pnpm-lock.yaml" ]]; then PKG_MGR="pnpm"
        elif [[ -f "$WEB_DIR/yarn.lock"       ]]; then PKG_MGR="yarn"
        else                                           PKG_MGR="npm"; fi

        check_tool "$PKG_MGR" "https://nodejs.org"
        check_tool node
        log_info "Node $(node --version) | $PKG_MGR"

        log_step "نصب وابستگی‌ها..."
        (cd "$WEB_DIR" && "$PKG_MGR" install)

        BUILD_SCRIPT="build"
        grep -q '"build:prod"' "$WEB_DIR/package.json" 2>/dev/null && BUILD_SCRIPT="build:prod"

        log_step "بیلد وب پنل ($BUILD_SCRIPT)..."
        T0=$(date +%s)
        (cd "$WEB_DIR" && "$PKG_MGR" run "$BUILD_SCRIPT")
        log_ok "وب پنل در $(($(date +%s) - T0))s بیلد شد."

        WEB_OUT=""
        for c in dist build out; do
            [[ -d "$WEB_DIR/$c" ]] && WEB_OUT="$WEB_DIR/$c" && break
        done

        if [[ -n "$WEB_OUT" ]]; then
            rm -rf "$DIST_DIR/web"
            cp -r "$WEB_OUT" "$DIST_DIR/web"
            log_ok "→ dist/web/"
        else
            log_warn "پوشه خروجی وب پنل (dist/build/out) پیدا نشد."
        fi
    fi
fi

# ══════════════════════════════════════════════════════════════════════════════
#  مرحله ۳ — فایل‌های کمکی
# ══════════════════════════════════════════════════════════════════════════════
log_section "مرحله ۳ — فایل‌های کمکی"

BIN_FINAL="${BIN_NAME:-tor-router}"

# ─── run.sh ───────────────────────────────────────────────────────────────────
cat > "$DIST_DIR/run.sh" << RUNEOF
#!/usr/bin/env bash
# اجرای Tor Router
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

# ─── خلاصه ────────────────────────────────────────────────────────────────────
log_section "نتیجه بیلد"
echo -e "${GREEN}✅ بیلد موفق!${RESET}\n"
echo -e "📦 محتوای dist/:"
ls -lh "$DIST_DIR"
echo ""
echo -e "${BOLD}${CYAN}اجرا:${RESET}"
echo -e "  ${CYAN}cd $DIST_DIR && ./run.sh${RESET}"
echo ""
echo -e "${BOLD}${CYAN}یا مستقیم:${RESET}"
echo -e "  ${CYAN}$DIST_DIR/$BIN_FINAL --run${RESET}"
echo -e "  ${CYAN}$DIST_DIR/$BIN_FINAL --web-dir ./web${RESET}"
echo ""
