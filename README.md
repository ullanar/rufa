# RUFA

**Rust Fitting Assistant.** An EVE Online fitting tool, like Pyfa, but blazingly fast ⚡ and super memory efficient 🧠.

> This project is fully vibe-coded. I don't have enough free time to write it by hand, so an AI does the typing and I do the thinking. It is here for fun and for the community. We still hold it to the practices of experienced developers: tests, clippy, pinned CI, no nonsense.

The attribute math is [dogma-engine](https://github.com/EVEShipFit/dogma-engine) by [TrueBrain](https://github.com/TrueBrain), the author of [EVEShipFit](https://github.com/EVEShipFit). Thank you for the engine and the patched SDE; none of this would exist without them. Rufa wraps the engine in a desktop app and adds the rest: graphs, capacitor simulation, projections, prices, characters from ESI, saved fits.

## Run

Download a build from [Releases](https://github.com/ullanar/rufa/releases) and start it. There is nothing else to install.

- Linux: `rufa-*-x86_64-unknown-linux-gnu`, built against glibc 2.28, so it runs on anything newer.
- Windows: `rufa-*-x86_64-pc-windows-msvc.exe`.
- macOS: `rufa-*-aarch64-apple-darwin` on Apple silicon, `rufa-*-x86_64-apple-darwin` on Intel.

Every target also ships as an archive next to the bare file: `.zip` on Windows, `.tar.gz` on Linux and macOS. On Linux and macOS mark the file executable once:

```
chmod +x rufa-*
```

The first start downloads the game data from the latest [sde-patched](https://github.com/EVEShipFit/sde-patched) release. Pass a fit file to open it straight away:

```
rufa my-fit.eft
```

### From source

Rust nightly, then:

```
cargo run --release
cargo run --release -- my-fit.eft
```

## Keys and mouse

- Ctrl+V pastes an EFT or DNA fit, Ctrl+C copies the open fit as EFT.
- Ctrl+S saves, Ctrl+Z and Ctrl+Y undo and redo, Ctrl+W closes the tab.
- Left click on a module state cycles it, right click overloads, middle click offlines.

Fits are esf/1 text files under `$XDG_CONFIG_HOME/rufa/fits`. Game data lives in `$XDG_DATA_HOME/rufa`.

## Contributing

Open a pull request against `master`. Every commit message follows [Conventional Commits](https://www.conventionalcommits.org/); the branch ruleset and CI reject anything else, and release-please turns the messages into the changelog.

## License

MIT. Game data belongs to Fenris Creations and ships under their license with [sde-patched](https://github.com/EVEShipFit/sde-patched).
