#!/usr/bin/env bash
set -uo pipefail

# Unified packaging script for noctyrn
# Builds release binaries for Windows and/or Linux, strips if requested, and creates zip artifacts

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

# Configuration
BUILD_WINDOWS=0 BUILD_LINUX=0 STRIP=0 SHOW_INFO=0 VERBOSE=0 CLEAN_DIST=0

# Results storage (associative arrays)
declare -A BUILD_INFO   # platform -> "build_time:pkg_time:orig_size:final_size:strip_done"
declare -A ZIP_INFO     # zip_path -> "zip_size:file_count"

show_help() {
  cat <<EOF
Usage: $0 [OPTIONS]

OPTIONS:
  -h, --help              Show this help message
  -W, --windows           Build Windows release
  -L, --linux             Build Linux release
  -A, --all               Build both Windows and Linux (same as -W -L)
  -S, --strip             Strip binaries to reduce size (default: off)
  -I, --info              Show detailed packaging info
  -v, --verbose           Verbose output
  -C, --clean-dist        Clean dist directory before building

EXAMPLES:
  $0 --all --strip                    Build both platforms with stripping
  $0 -WL -S --info                    Build W+L, strip, show info
  $0 -A                               Build both platforms (no strip)
  $0 -L --strip --info                Build Linux with strip and detailed info

EOF
}

# Parse combined short flags like -WLS
parse_combined_flags() {
  local arg="$1"
  for (( i=1; i<${#arg}; i++ )); do
    case "${arg:$i:1}" in
      W) BUILD_WINDOWS=1 ;;
      L) BUILD_LINUX=1 ;;
      A) BUILD_WINDOWS=1; BUILD_LINUX=1 ;;
      S) STRIP=1 ;;
      I) SHOW_INFO=1 ;;
      v) VERBOSE=1 ;;
      C) CLEAN_DIST=1 ;;
      *) echo "Unknown option: -${arg:$i:1}"; exit 1 ;;
    esac
  done
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) show_help; exit 0 ;;
    -[WLASIvC]*) parse_combined_flags "$1" ;;
    -W|--windows) BUILD_WINDOWS=1 ;;
    -L|--linux) BUILD_LINUX=1 ;;
    -A|--all) BUILD_WINDOWS=1; BUILD_LINUX=1 ;;
    -S|--strip) STRIP=1 ;;
    -I|--info) SHOW_INFO=1 ;;
    -v|--verbose) VERBOSE=1 ;;
    -C|--clean-dist) CLEAN_DIST=1 ;;
    *) echo "Unknown option: $1"; show_help; exit 1 ;;
  esac
  shift
done

# Validate platform selection
[ "$BUILD_WINDOWS" -eq 0 ] && [ "$BUILD_LINUX" -eq 0 ] && \
  { echo "Error: No platform selected. Use -W, -L, or -A"; show_help; exit 1; }

# Utility functions
log() { [ "$VERBOSE" -eq 1 ] && echo "[*] $*"; }
bytes_to_human() { 
  local bytes=$1
  if (( bytes < 1024 )); then echo "${bytes}B"
  elif (( bytes < 1024*1024 )); then echo "$((bytes / 1024))K"
  elif (( bytes < 1024*1024*1024 )); then echo "$((bytes / (1024*1024)))M"
  else echo "$((bytes / (1024*1024*1024)))G"; fi
}
get_file_count() { unzip -l "$1" 2>/dev/null | tail -1 | awk '{print $2}' || echo "?"; }

# Build and package a platform
build_platform() {
  local platform="$1" target="$2" out_dir="$3" binary_name="$4"
  
  echo "=========================================="
  echo "Building $platform release..."
  echo "=========================================="
  
  local build_start=$(date +%s)
  
  if [ "$platform" = "Windows" ]; then
    log "Adding Windows target..."
    rustup target add "$target" 2>/dev/null || true
    log "Configuring Windows cross-compilation environment..."
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="x86_64-w64-mingw32-gcc"
    export CC_x86_64_pc_windows_gnu="x86_64-w64-mingw32-gcc"
    export CXX_x86_64_pc_windows_gnu="x86_64-w64-mingw32-g++"
    # Ensure MinGW binaries are in PATH and libraries are in LDFLAGS if provided by Nix
    # Use NOCTYRN_* envvars (set by flake.nix) if available. Older name STREAK_*
    # was a typo and may be present on some systems — support both for compatibility.
    if [ -n "${NOCTYRN_WINDOWS_DEPS:-}" ]; then
      for dep in $NOCTYRN_WINDOWS_DEPS; do
        export PATH="$dep/bin:$PATH"
      done
    elif [ -n "${STREAK_WINDOWS_DEPS:-}" ]; then
      for dep in $STREAK_WINDOWS_DEPS; do
        export PATH="$dep/bin:$PATH"
      done
    fi

    if [ -n "${NOCTYRN_WINDOWS_LDPATH:-}" ]; then
      export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-L native=$NOCTYRN_WINDOWS_LDPATH"
    elif [ -n "${STREAK_WINDOWS_LDPATH:-}" ]; then
      export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-L native=$STREAK_WINDOWS_LDPATH"
    fi
    log "Building with cargo..."
  fi
  
  if [ "$platform" = "Linux" ]; then
    # Unset cross-compilation variables that might have been set by the Nix shell
    # to ensure a clean native build.
    (unset CC CXX LD AR; cargo build --release --no-default-features)
  else
    cargo build --release ${target:+--target "$target"} --no-default-features
  fi
  
  local build_end=$(date +%s)
  local dist_dir="dist/${platform,,}"
  rm -rf "$dist_dir"
  mkdir -p "$dist_dir"
  
  # Copy binary and assets (strip runtime data first)
  rm -f settings/auth_token.json settings/savestate.json 2>/dev/null || true
  cp "$out_dir/$binary_name" "$dist_dir/"
  cp -r assets settings README.md "$dist_dir/" 2>/dev/null || true
  
  local orig_size=$(stat -c%s "$out_dir/$binary_name" 2>/dev/null || echo 0)
  local bin_path="$dist_dir/$binary_name"
  
  # Strip if requested
  local strip_done=0
  if [ "$STRIP" -eq 1 ]; then
    log "Attempting to strip $platform binary..."
    if [ "$platform" = "Windows" ]; then
      if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
        x86_64-w64-mingw32-strip "$bin_path" 2>/dev/null || true
        strip_done=1
      elif command -v i686-w64-mingw32-strip >/dev/null 2>&1; then
        i686-w64-mingw32-strip "$bin_path" 2>/dev/null || true
        strip_done=1
      elif command -v strip >/dev/null 2>&1; then
        strip "$bin_path" 2>/dev/null || true
        strip_done=1
      fi
      # Copy MinGW runtime DLLs
      log "Copying MinGW runtime DLLs..."
      local mingw_paths=(/usr/x86_64-w64-mingw32/sys-root/mingw/bin /usr/i686-w64-mingw32/sys-root/mingw/bin)
      for p in "${mingw_paths[@]}"; do
        [ -d "$p" ] && cp "$p"/{libwinpthread-1,libgcc_s_seh-1,libstdc++-6}.dll "$dist_dir/" 2>/dev/null || true
      done
    else
      if command -v strip >/dev/null 2>&1; then
        strip "$bin_path" 2>/dev/null || true
        strip_done=1
      fi
    fi
  fi
  
  local final_size=$(stat -c%s "$bin_path" 2>/dev/null || echo 0)
  
  local pkg_start=$(date +%s)
  
  # Create zip
  local version=$(cargo pkgid | sed 's/.*#//')
  local zip_name="noctyrn_${platform,,}_v${version}_$(date +%y-%m-%d@%H:%M).zip"
  (cd dist && zip -q -r "$zip_name" "${platform,,}")
  
  local pkg_end=$(date +%s)
  local zip_path="dist/$zip_name"
  local zip_size=$(stat -c%s "$zip_path" 2>/dev/null || echo 0)
  local file_count=$(get_file_count "$zip_path")
  
  # Store results
  local build_time=$((build_end - build_start))
  local pkg_time=$((pkg_end - pkg_start))
  BUILD_INFO["$platform"]="$build_time:$pkg_time:$orig_size:$final_size:$strip_done"
  ZIP_INFO["$zip_path"]="$zip_size:$file_count"
}

# Main execution
OVERALL_START=$(date +%s)


[ "$CLEAN_DIST" -eq 1 ] && { log "Cleaning dist directory..."; rm -rf dist/*; }

[ "$BUILD_LINUX" -eq 1 ] && build_platform "Linux" "" "target/release" "noctyrn"
[ "$BUILD_WINDOWS" -eq 1 ] && build_platform "Windows" "x86_64-pc-windows-gnu" "target/x86_64-pc-windows-gnu/release" "noctyrn.exe"

OVERALL_END=$(date +%s)
OVERALL_TIME=$((OVERALL_END - OVERALL_START))

# Print comprehensive summary
echo ""
echo "=========================================="
echo "BUILD SUMMARY"
echo "=========================================="

for platform in "Linux" "Windows"; do
  [ "${BUILD_INFO[$platform]:-}" ] || continue
  IFS=: read build_time pkg_time orig_size final_size strip_done <<< "${BUILD_INFO[$platform]}"
  total_time=$((build_time + pkg_time))
  
  echo ""
  echo "  $platform:"
  echo "    Build time: ${build_time}s | Packaging time: ${pkg_time}s | Total: ${total_time}s"
  echo "    Binary size: $(bytes_to_human "$orig_size") -> $(bytes_to_human "$final_size") ($orig_size bytes)"
  [ "$SHOW_INFO" -eq 1 ] && echo "    Install size: $(du -sh "dist/${platform,,}" 2>/dev/null | cut -f1 || echo "0")"
  echo "    Stripped: $([ "$strip_done" -eq 1 ] && echo "yes" || echo "no")"
  
done

echo ""
echo "=========================================="
echo "PACKAGE SUMMARY"
echo "=========================================="
echo "Overall time: ${OVERALL_TIME}s"
echo "Platforms: $([ "$BUILD_WINDOWS" -eq 1 ] && echo -n "Windows " || true)$([ "$BUILD_LINUX" -eq 1 ] && echo "Linux" || true)"
echo "Strip enabled: $([ "$STRIP" -eq 1 ] && echo "yes" || echo "no")"

echo ""
echo "Archives created:"
for zip_path in "${!ZIP_INFO[@]}"; do
  IFS=: read zip_size file_count <<< "${ZIP_INFO[$zip_path]}"
  echo "  • $(basename "$zip_path")"
  echo "    Size: $(bytes_to_human "$zip_size") ($zip_size bytes) | Files: $file_count"
  [ "$SHOW_INFO" -eq 1 ] && echo "    Path: $zip_path"
done

echo ""
echo "Packaging complete!"
