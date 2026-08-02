# Klara

Klara is a terminal emulator for Apple-silicon Macs running macOS 15 or newer.
It is currently under development and is not yet ready to replace a production
terminal.

Development is primarily tested on macOS 26.5.2. Compatibility with other
supported macOS versions is best-effort until it has been verified in a clean
macOS 15 environment.

## Build from source

Klara requires:

- An Apple-silicon Mac running macOS 15 or newer.
- The Xcode command-line tools.
- Rust 1.85 or newer.

Install the Xcode command-line tools if they are not already installed:

```sh
xcode-select --install
```

Install Rust using [rustup](https://rustup.rs/), then build and run Klara:

```sh
cargo run --release
```

The Cargo configuration builds for `aarch64-apple-darwin` with a macOS 15
deployment target.

## Distribution

Initial releases are built from source and require no paid developer services.
Developer ID signing, notarization, DMG packaging, Homebrew distribution, and
automatic updates are deferred.

See [PRODUCTION_HARDENING.md](PRODUCTION_HARDENING.md) for the implementation
roadmap and [docs/TERMINAL_COMPATIBILITY.md](docs/TERMINAL_COMPATIBILITY.md) for
the current terminal protocol coverage.
