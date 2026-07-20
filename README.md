# Overview
This game is a goofy, low poly, tactical first person shooter designed to fill the gap left by phantom forces, siege, and other games.

## Architecture

```
src/
├── main.rs              # App entry point, plugin registration
├── player/              # Player controller, movement, camera, input, shooting
│   ├── mod.rs           # GameState enum (MainMenu, Playing, Login, Profile, etc.)
│   ├── movement/        # Physics-based FPS movement (walk, sprint, jump, crouch, slide)
│   ├── camera.rs        # First-person camera with recoil and sway
│   ├── input.rs         # Input accumulation and toggle states
│   └── shooting.rs      # Hitscan, projectiles, muzzle flash, tracers
├── weapons/             # Weapon registry, loadouts, skins, attachments
├── menu.rs              # All menu screens (main menu, loadout, game mode, login,
│                        #   profile, friends, lobby, matchmaking, crate opening, cosmetics)
├── gameplay.rs          # Match state, scoring, kill feed, objectives
├── gamemodes/           # Per-mode logic (FFA, TDM, Kill Confirmed, CTF, etc.)
├── world/               # Map loading, static colliders, pickup spawning
├── net/                 # Multiplayer networking
│   ├── mod.rs           # NetworkPlugin, ConnectionState, NetworkEvent messages
│   ├── http.rs          # Async HTTP client (login, register, profile, friends, matchmaking)
│   ├── tcp.rs           # TCP client for lobby/matchmaking push updates
│   ├── udp.rs           # UDP client for real-time gameplay input/state sync
│   ├── prediction.rs    # Client-side prediction buffer with server reconciliation
│   └── interpolation.rs # Entity interpolation for smooth remote player rendering
├── ui_settings.rs       # Settings menu (sensitivity, FOV, keybindings, audio, video)
└── ui_config.rs         # HUD layout configuration (health bar, ammo, kill feed)
```

## Multiplayer

The game client connects to `noctyrn-server` for online play:
- **HTTP** (port 8080) - Authentication, profile, friends, matchmaking queue
- **TCP** (port 7878) - Lobby state push updates
- **UDP** (port 7877) - Real-time gameplay input/state sync

### Connecting to a server

Edit `assets/server.toml` to point to your server before launching the game:

```toml
[server]
http_url = "http://YOUR_SERVER_IP:8080"
tcp_addr  = "YOUR_SERVER_IP:7878"
udp_addr  = "YOUR_SERVER_IP:7877"
```

If the file is missing or can't be parsed, the game falls back to `127.0.0.1` (localhost).

**Home PC hosting checklist:**
- Forward ports **8080 (TCP)**, **7878 (TCP)**, and **7877 (UDP)** on your router to the machine running the server
- Find your public IP at [whatismyip.com](https://whatismyip.com) and put that in `server.toml`
- Home ISPs sometimes change your public IP — a free DDNS service like [DuckDNS](https://www.duckdns.org/) gives you a stable hostname that auto-updates

**When you get a domain:** just update `server.toml` to use `https://game.yoursite.com` etc. Nothing else changes in the code.

## Game Modes

FreeForAll, TeamDeathmatch, KillConfirmed, CaptureTheFlag, Assassins, KingOfTheHill, Hardpoint, CapturePoint, TestingGrounds, plus limited-time modes (Juggernaut, HighExplosives, OneInTheChamber, GunGame, Infected).

## Dependencies

This crate is part of a [Cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html). It depends on `noctyrn-shared` (path dependency via `../noctyrn-shared`). The workspace root `Cargo.toml` is at `../Cargo.toml`.

Key Rust dependencies:
- `bevy 0.18` - Game engine
- `noctyrn-shared` - Shared protocol types, player/lobby/weapon definitions
- `reqwest 0.12` - Async HTTP client for server API
- `tokio 1` - Async runtime for networking
- `serde` / `serde_json` - Serialization
- `uuid` - Player and entity IDs


# Build Instructions

## NixOS / Nix (recommended)

```bash
cd noctyrn-game
nix develop
cargo run
```

## Manual setup
1. Make sure you have [Rust](https://www.rustup.rs) installed (stable toolchain).
2. Clone the repository: `git clone https://github.com/gitanelyon/noctyrn.git`
3. Navigate to the project directory: `cd noctyrn`
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

The produced binary will be at `target/release/noctyrn` (Linux) or `target/x86_64-pc-windows-gnu/release/noctyrn.exe` for a cross-built Windows binary.

**Run the release build**

```bash
# run the static release
cargo run --release --no-default-features

# or run executable directly
./target/release/noctyrn
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

The script creates zip files in the `dist/` directory with timestamped names (e.g., `noctyrn_linux_v0.1.0_25-12-15@20:11.zip`).

# Model credits
Most gun models are from [Sketchfab] from creates like favoritelike69, D_U, TastyTony, GoldbergR, notcplkerry, and others, are licensed under CC-BY-4.0