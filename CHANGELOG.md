# Changelog

## [0.8.0](https://github.com/ullanar/rufa/compare/v0.7.0...v0.8.0) (2026-11-09)


### Features

* **app:** the graph pane draws all nine Pyfa graphs ([f5c3e84](https://github.com/ullanar/rufa/commit/f5c3e84ace2f4934cf014fd4151cdab63ac82fb0))
* **sim:** damage stats graph math with targets, tackle and time ([65cfaa0](https://github.com/ullanar/rufa/commit/65cfaa0c9fcb2692b644a709755d730b88657d53))
* **sim:** the other eight graphs and one entry point for all nine ([f22dca4](https://github.com/ullanar/rufa/commit/f22dca42a58a2b65e897808687dcbbe716fa6a5d))


### Bug Fixes

* **app:** an opened fit sheds the modules its hull cannot take ([d7073d3](https://github.com/ullanar/rufa/commit/d7073d35e4fe65ec287983915c3abd85047f27f8))
* **app:** Ctrl+Shift+G steps the graph with the shifted key ([c7546e1](https://github.com/ullanar/rufa/commit/c7546e1b217ba1bb4030a121f9cd010c0006ef58))
* **app:** the effective damage axis keeps DPS in capitals ([334fd5f](https://github.com/ullanar/rufa/commit/334fd5f067e7c4127fcd3d14220da4c179513bac))
* **app:** the status bar clips a message instead of running under the zoom control ([33ba05d](https://github.com/ullanar/rufa/commit/33ba05dbbc5b7c8abf756c782a09249e8b81183c))

## [0.7.0](https://github.com/ullanar/rufa/compare/v0.6.1...v0.7.0) (2026-11-06)


### Features

* **app:** a second Pyfa import brings only the new fits ([c2f5c0f](https://github.com/ullanar/rufa/commit/c2f5c0fcbe633f6c7171d9c213e4d4d433d82e6e))
* **app:** every row of a fit names its price ([4a383f7](https://github.com/ullanar/rufa/commit/4a383f72b8d82871eefb6c1a01de4cfeb8852fa3))
* **app:** find the duplicate fits and delete them in one step ([33e5006](https://github.com/ullanar/rufa/commit/33e5006fdc651132b70ae1db093275d7b1934c90))
* **app:** the window comes back the size and place it was left ([33bdd7c](https://github.com/ullanar/rufa/commit/33bdd7c125910fe26d42d606455d84656d81d3fe))
* **sim:** a projected transmitter feeds the capacitor run ([8072eea](https://github.com/ullanar/rufa/commit/8072eeac18fa93cd22bb947fedd9f24fc88a0662))
* **sim:** cap boosters inject and a clip reloads in the capacitor run ([d3f48cd](https://github.com/ullanar/rufa/commit/d3f48cd9aff3a38c51d22b0aa99de8fb47616b78))
* **store:** damage patterns come from a file, not a fixed list ([1c45971](https://github.com/ullanar/rufa/commit/1c45971094045f114f7853079054e91d6729d8af))


### Bug Fixes

* **app:** a fit row measures its text in pixels, not characters ([28cb8c1](https://github.com/ullanar/rufa/commit/28cb8c1954d9052954038ff46b7790b8d983a361))
* **app:** a running Pyfa import refuses a second press ([c79621f](https://github.com/ullanar/rufa/commit/c79621f9def740eef07d6d7c25bb3621190753dd))
* **app:** the stats pane folds a row only when it stops fitting ([63a8b79](https://github.com/ullanar/rufa/commit/63a8b7900cd8ddc47581f453d83022f41cbb0a21))
* **app:** zoom walks a ladder and a fresh window fits the monitor ([ac7650a](https://github.com/ullanar/rufa/commit/ac7650a90bb8a03686e72de1edb206b67bcb81b2))

## [0.6.1](https://github.com/ullanar/rufa/compare/v0.6.0...v0.6.1) (2026-11-02)


### Bug Fixes

* **app:** the damage total counted drones and fighters twice ([8056ada](https://github.com/ullanar/rufa/commit/8056ada35b893032ecf906b35b9f68ea2b2dddab))

## [0.6.0](https://github.com/ullanar/rufa/compare/v0.5.0...v0.6.0) (2026-11-01)


### Features

* **app:** a fit opens in front of the tabs already there ([5768662](https://github.com/ullanar/rufa/commit/5768662e0c2d163f343d22c87f3834545534537c))
* **app:** click an empty slot to search the market for that rack ([ce72981](https://github.com/ullanar/rufa/commit/ce72981f868f5608d877539543ecb3a172745c77))
* **app:** fold a stat section, its headline stays ([22eebcf](https://github.com/ullanar/rufa/commit/22eebcf39f8aa9f1d74e52a553c6e8a8d70c473e))
* **app:** one unit toggle and one capacitor verdict in the stats pane ([b9dca61](https://github.com/ullanar/rufa/commit/b9dca61d73405a67f4c04736d62db920d8e749ff))
* **app:** tell the fit price from the price in the head ([d734e4f](https://github.com/ullanar/rufa/commit/d734e4f3557f7b81154c0714dd9103a8dda697c5))
* **app:** the loaded charge shows its icon ([2362118](https://github.com/ullanar/rufa/commit/23621185de717fd6cbc35cd039c5bfb4701aa1eb))
* **app:** the saved fits read as a tree, with copy and delete on the row ([9c1697e](https://github.com/ullanar/rufa/commit/9c1697ec050e7acbfd304b179d71df486a57585b))
* **app:** type a cargo or drone count instead of clicking plus ([abd4b45](https://github.com/ullanar/rufa/commit/abd4b453c28d07312aeaecfccf10caf225d0ffa6))
* **app:** type icons in the ship, saved fit and market lists ([2ef76fb](https://github.com/ullanar/rufa/commit/2ef76fbd438112d5de8d4fa697a2d94b0dae2f9e))
* stow anything in the cargo hold, where it counts for nothing ([ae76742](https://github.com/ullanar/rufa/commit/ae76742f6e09732453433c5c1d68c6e958a174e1))


### Bug Fixes

* **app:** a counted row names the item without repeating the count ([e9ba7c4](https://github.com/ullanar/rufa/commit/e9ba7c4de014b1b915eddb3755bea32c2a86bfe6))

## [0.5.0](https://github.com/ullanar/rufa/compare/v0.4.0...v0.5.0) (2026-11-01)


### Features

* **app:** a Start Menu shortcut and a Path entry on windows ([d1734c9](https://github.com/ullanar/rufa/commit/d1734c9ce7bb0ab557dd018f7ff15eb966ae93cf))
* **app:** give the window an icon ([fe106e4](https://github.com/ullanar/rufa/commit/fe106e4b9a5fc759c0202173d3a61686d3aec0ac))
* **app:** install the downloaded build into a per user directory ([33df7b2](https://github.com/ullanar/rufa/commit/33df7b2a72ea260c8921aecff9e5ab3e482f1e86))
* **app:** pin the violations strip above the status bar ([7d020d2](https://github.com/ullanar/rufa/commit/7d020d2d580d86921145b0ef7f25b3605ea773de))
* offer the newer release and swap the running build ([b310d60](https://github.com/ullanar/rufa/commit/b310d60a53a15486a5b584ae706ec3472de2b463))


### Bug Fixes

* **app:** any cargo target directory counts as a build tree ([4826544](https://github.com/ullanar/rufa/commit/48265440bc23300cfff53fd80d7c4079fdbf94b3))
* **app:** any cargo target directory counts as a build tree ([9ad2504](https://github.com/ullanar/rufa/commit/9ad2504751442b3e4c282a6cf4102ca30e974bfc))
* **app:** keep the group count clear of a long group name ([2b395ea](https://github.com/ullanar/rufa/commit/2b395eaa3b2b9cb8ede6edd297ae3ccfc5698710))
* **app:** the window application id is a Linux only field ([a5107bd](https://github.com/ullanar/rufa/commit/a5107bd19d0c4a0276dac9e448ec67e4eb8e9a2b))
* **store:** keep the ui scale across restarts ([c1c63a7](https://github.com/ullanar/rufa/commit/c1c63a7d314a8c84b9e7b91ad78b7d8f86d5d15e))

## [0.4.0](https://github.com/ullanar/rufa/compare/v0.3.0...v0.4.0) (2026-11-01)


### Features

* **app:** drone rows, fits tab and selection-wide charges ([333d562](https://github.com/ullanar/rufa/commit/333d562fd20ec0019969bbb0bafa8d4863b45800))
* **app:** incoming reps, jam chance, dps range and bomb rows ([1635e52](https://github.com/ullanar/rufa/commit/1635e52a2260f39ec34f0c38501bd83eee6d5998))
* **app:** remote reps and mining sections, empty rows hidden ([4d60c9c](https://github.com/ullanar/rufa/commit/4d60c9ca95fa2498d823a189227e5cc4342b7431))
* **sim:** drone bay limits and the stats they feed ([dad1048](https://github.com/ullanar/rufa/commit/dad1048badbedc82683e9e73f533f20ffaa11598))
* **sim:** incoming reps, ECM jam chance, spooled dps and bombs to kill ([ac9506f](https://github.com/ullanar/rufa/commit/ac9506f3d536587566d7f7bfe09ef212c242b73c))
* **sim:** remote reps, sustained tank, mining yield and damage split ([3d3cb67](https://github.com/ullanar/rufa/commit/3d3cb67f7350580641d4b25fc9f2f55adf598a68))

## [0.3.0](https://github.com/ullanar/rufa/compare/v0.2.0...v0.3.0) (2026-11-01)


### Features

* first-run screen with per-file progress and a spinner, state clicks act on the selection ([04933f8](https://github.com/ullanar/rufa/commit/04933f80cd52252928b030ce7183547e76141075))
* hover highlight on module rows, state tooltips and Escape to deselect ([1f6135b](https://github.com/ullanar/rufa/commit/1f6135bc9a2e2fcff62b2e7c072c9b794553a7c2))
* import every fit from a Pyfa database with a built-in SQLite reader ([e7d9664](https://github.com/ullanar/rufa/commit/e7d9664fc7e0c934b118949981bf5aa3326306e7))
* larger type, higher contrast, UI zoom with Ctrl plus and minus, grouped variations ([7525f08](https://github.com/ullanar/rufa/commit/7525f08ac787df60af2497a1c7e91a4847b70df8))
* multi-select with Ctrl and Shift and a context menu in Pyfa order ([0fd1180](https://github.com/ullanar/rufa/commit/0fd11804a7a3ae31666900536eac4d2827a4397b))
* own visual theme with embedded fonts, gauges, resist bars and state glyphs ([2bcd88a](https://github.com/ullanar/rufa/commit/2bcd88a8d7f91f66e6a7deb96c2d069ff6e1f084))
* refuse modules the hull cannot take instead of listing the violation ([0430d5f](https://github.com/ullanar/rufa/commit/0430d5f2e68208aa287d0918aa93cd42eecd866c))
* responsive stats pane with threshold colors, pane width guards, debounced settings save ([49b318c](https://github.com/ullanar/rufa/commit/49b318c351043e5031f232042cbe09f6f14e8626))
* tab flags for the left pane and wider name columns in the fit view ([668ccbd](https://github.com/ullanar/rufa/commit/668ccbdaf878325fd892b703f0ba2deff41f0436))
* type icons and hull renders from the EVE image server with a disk cache ([65e3eff](https://github.com/ullanar/rufa/commit/65e3eff45ca4d087a2262cbe2a700e31330147a1))


### Bug Fixes

* zoom shortcuts react to the physical key, including the numpad ([8b56ac5](https://github.com/ullanar/rufa/commit/8b56ac5fdda988c27b5bddc6f081637d41b4d8c8))

## [0.2.0](https://github.com/ullanar/rufa/compare/v0.1.0...v0.2.0) (2026-11-01)


### Features

* initial release of Rufa, a Rust fitting assistant for EVE Online ([f57ed65](https://github.com/ullanar/rufa/commit/f57ed65dca57039cd40500f134ded491d82127c2))
