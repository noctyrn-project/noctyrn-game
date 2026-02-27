# Overview
This game is a goofy, low poly, tactical first person shooter designed to fill the gap left by phantom forces, siege, and other games.


# Build Instructions
1. Make sure you have [Rust](https://www.rustup.rs) installed (stable toolchain).
2. Clone the repository: `git clone https://github.com/gitanelyon/fearlyss.git`
3. Navigate to the project directory: `cd fearlyss`
4. Build the project (development): `cargo run`

Note: this project uses Bevy with dynamic linking during development to keep iteration quick. For a distributable single-binary release we build without the dynamic linking feature (see Release below).

**Development (fast iteration)**

> Run in development mode (dynamic linking enabled by default):

```bash
cargo run
```

**Release (single-binary, recommended for distribution)**

> Build a release binary that does not use Bevy's dynamic linking (produces a single executable):

```bash
# build release (static wrt Bevy dylib)
cargo build --release --no-default-features

# or use the provided cargo alias
cargo build-release
```

The produced binary will be at `target/release/fearlyss` (Linux) or `target/x86_64-pc-windows-gnu/release/fearlyss.exe` for a cross-built Windows binary.

**Run the release build**

```bash
# run the static release
cargo run --release --no-default-features

# or run executable directly
./target/release/fearlyss
```

If you previously ran in development mode and saw errors like `error while loading shared libraries: libbevy_dylib-...so: cannot open shared object file`, building with `--no-default-features` (release) avoids that because the Bevy dynamic library is not used.

**Cargo aliases**

The repository contains `.cargo/config.toml` with helpful aliases:

- `cargo run-dev` -> development run (same as `cargo run`)
- `cargo build-release` -> `cargo build --release --no-default-features`
- `cargo run-release` -> `cargo run --release --no-default-features`

Use whichever you prefer.

**Packaging scripts**

A unified packaging script is provided in `scripts/package.sh` that handles building and packaging for Linux, Windows, or both with flexible options.

- `-W, --windows` - Build Windows release
- `-L, --linux` - Build Linux release
- `-A, --all` - Build both Windows and Linux
- `-S, --strip` - Strip binaries to reduce size
- `-I, --info` - Show detailed packaging info (file counts, sizes, paths)
- `-v, --verbose` - Verbose output during build
- `-C, --clean-dist` - Clean dist directory before building
- `-h, --help` - Show help message

**Example usage**

```bash
# Build to all platforms with stripping and info
./scripts/package.sh -ASI
```

The script creates zip files in the `dist/` directory with timestamped names (e.g., `fearlyss_linux_v0.1.0_25-12-15@20:11.zip`).

# Model credits
Most gun models are from [Sketchfab] from creates like favoritelike69, D_U, TastyTony, GoldbergR, notcplkerry, and others, are licensed under CC-BY-4.0