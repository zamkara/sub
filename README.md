# sab sib seb sob sub

CLI tool written in Rust for subtitle management: list, extract, and inject subtitles from MKV files, as well as search and download subtitles from [SubSource.net](https://subsource.net).

## Features

- Search subtitles from SubSource.net by movie/show title
- Download subtitles (auto-extract ZIP to SRT)
- List all subtitle tracks in a video file
- Extract subtitle from MKV to separate SRT file
- Inject subtitle into MKV file
- Default language prioritization in search results
- Configurable default language and output directory
- API key management

## Install

### Requirements

- **Rust toolchain** (to build from source)
- **mkvtoolnix** (`mkvmerge`, `mkvextract`) - required for MKV operations (list/extract/inject)

```bash
# Arch Linux
sudo pacman -S mkvtoolnix

# Debian/Ubuntu
sudo apt install mkvtoolnix
```

The search and download features do not require any external dependencies beyond the compiled binary itself.

### Quick Install

```bash
# Build and install to ~/.local/bin
./build.sh install

# Or deploy directly
./deploy.sh user       # Install to ~/.local/bin
./deploy.sh system     # Install to /usr/local/bin (requires sudo)
```

### Build from Source

```bash
# Development build
./build.sh dev

# Release build (optimized)
./build.sh release

# Clean build artifacts
./build.sh clean
```

## Usage

### API Key Setup

First, configure your API key from SubSource.net:

```bash
sub key setup
```

Or set via environment variable:

```bash
export SUBSOURCE_API_KEY=your_api_key
```

### Configuration

```bash
# Show current config
sub config show

# Set default language
sub config set lang indonesian

# Set default output directory
sub config set dir ~/Downloads

# Reset to defaults
sub config reset
```

Supported languages: `indonesian`, `english`, `french`, `spanish`, `japanese`, `korean`, `chinese`, `malay`, `thai`, `vietnamese`, `arabic`, `portuguese`, `german`, `italian`, `russian`

### Search and Download

```bash
# Search subtitles (interactive)
sub search "Hoppers 2026"

# Search with year filter
sub search "Hoppers" -y 2026

# Search for English subtitles
sub search "Hoppers" -y 2026 -l english

# Non-interactive (auto-select first)
sub search "Hoppers" -y 2026 -l indonesian -n

# Verbose mode (print raw API response)
sub search "Hoppers" -v

# Download to specific directory
sub search "Hoppers" -o /tmp
```

In interactive mode, you can:
- Type a number to select a specific subtitle (e.g. `1,3,5`)
- Type a range (e.g. `1-3`)
- Type `all` to download all subtitles
- Press Enter to select the default (first result)

### Download by ID

```bash
sub download <movie_id> <subtitle_id> -o /tmp
```

### MKV Operations

**List subtitle tracks:**
```bash
sub list movie.mkv
```

Output marks the default language tracks with a star symbol.

**Extract subtitle:**
```bash
sub extract movie.mkv -i 2
sub extract movie.mkv -i 2 -o subs.srt
```

**Inject subtitle:**
```bash
sub inject movie.mkv subtitle.srt
sub inject movie.mkv subtitle.srt -l ind -n Indonesian
sub inject movie.mkv subtitle.srt -l eng -n "English [SDH]"
```

### API Key Management

```bash
sub key setup     # Configure API key (interactive)
sub key add <key> # Add/update API key directly
sub key show      # Show API key (masked)
sub key remove    # Remove stored API key
```

## Commands Reference

| Command | Description |
|---------|-------------|
| `sub list <video>` | List all subtitle tracks in video |
| `sub extract <video> -i <id>` | Extract subtitle track to SRT file |
| `sub inject <video> <subtitle>` | Inject subtitle into MKV file |
| `sub search <query>` | Search and download subtitles from SubSource |
| `sub download <movie_id> <sub_id>` | Download subtitle by ID |
| `sub key setup` | Setup API key |
| `sub config show` | Show current configuration |
| `sub config set lang <code>` | Set default language |
| `sub config set dir <path>` | Set default output directory |
| `sub config reset` | Reset all configuration |

## Options

### Extract

| Flag | Description |
|------|-------------|
| `-i <id>` | Track ID (required) |
| `-o <file>` | Output filename |

### Inject

| Flag | Description |
|------|-------------|
| `-l <lang>` | Language code (default: `ind`) |
| `-n <name>` | Track name |

### Search

| Flag | Description |
|------|-------------|
| `-l <lang>` | Language (override default) |
| `-y <year>` | Release year filter |
| `-o <dir>` | Output directory (override default) |
| `-n` | Non-interactive (auto-select first) |
| `-v` | Verbose |

## Project Structure

```
sub/
├── Cargo.toml           # Dependencies and build config
├── build.sh             # Build script
├── deploy.sh            # Deploy script
├── src/
│   ├── main.rs          # CLI entry point
│   ├── config.rs        # Config and API key management
│   ├── mkv.rs           # MKV operations
│   └── subsource.rs     # SubSource API client
└── target/              # Build artifacts
```

## Dependencies (Internal)

All dependencies are managed by Cargo (`Cargo.toml`):

- `clap` - CLI argument parsing
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde` / `serde_json` - JSON parsing
- `colored` - Terminal colors
- `zip` - ZIP extraction
- `tempfile` - Temporary files
- `dirs` / `home` - Home directory resolution

## External Dependencies

- `mkvtoolnix` - Required for `list`, `extract`, and `inject` commands. Not required for `search` and `download`.

## License

MIT

# Credits
- zamkara
- mbunkus
- Subsource
