# RUFA

**Rust Fitting Assistant.** An EVE Online fitting tool, like Pyfa, but blazingly fast ⚡ and super memory efficient 🧠.

> This project is fully vibe-coded. I don't have enough free time to write it by hand, so an AI does the typing and I do the thinking. It is here for fun and for the community. We still hold it to the practices of experienced developers: tests, clippy, pinned CI, no nonsense.

The attribute math is EVEShipFit's `esf-dogma-engine`. Rufa wraps it in a desktop app and adds the rest: graphs, capacitor simulation, projections, prices, characters from ESI, saved fits.

## Run

```
cargo run --release
```

The first start downloads the game data from the latest `sde-patched` release. Drop a fit file on the command line to open it:

```
cargo run --release -- my-fit.eft
```

## Keys and mouse

- Ctrl+V pastes an EFT or DNA fit, Ctrl+C copies the open fit as EFT.
- Ctrl+S saves, Ctrl+Z and Ctrl+Y undo and redo, Ctrl+W closes the tab.
- Left click on a module state cycles it, right click overloads, middle click offlines.

Fits are esf/1 text files under `$XDG_CONFIG_HOME/rufa/fits`. Game data lives in `$XDG_DATA_HOME/rufa`.

## License

MIT. Game data belongs to CCP and ships under their license with `sde-patched`.
