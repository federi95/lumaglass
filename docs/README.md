# Lumaglass documentation

Lumaglass is a native TV application derived from Luma. The guides below describe
the current implementation; the project excludes a web dashboard and MQTT bridge.

| Document                                          | Purpose                                                           |
| ------------------------------------------------- | ----------------------------------------------------------------- |
| [Getting started](getting-started.md)             | Local macOS setup, preview and ARM package                        |
| [Development](development.md)                     | Toolchain, command reference, checks and preview tooling          |
| [Controls](controls.md)                           | Remote, pointer, keyboard and recent-app navigation               |
| [Remote shortcuts](remote-shortcuts.md)           | Opt-in color-button actions, desktop aliases and reception status |
| [Home personalization](home-personalization.md)   | Greeting, date, section order and initial focus                   |
| [Appearance](appearance.md)                       | Card density and native visual preferences                        |
| [Personal wallpapers](wallpapers.md)              | Local/HTTPS images, offline cache and rotation                    |
| [Local screensaver](screensaver.md)               | App-local idle presentation and waking input                      |
| [Render resolution](display.md)                   | Experimental 4K trials and independent safe mode                  |
| [Pending TV verification](manual-verification.md) | Weekend hardware checklist                                        |
| [Privacy and blocker](privacy-and-blocker.md)     | Confirmed settings, optional hosts blocking and recovery          |
| [App management](app-management.md)               | Local hiding, confirmed uninstallation and verification rules     |
| [Architecture](architecture.md)                   | Native modules, platform workers, storage and project boundaries  |
| [TV integration](tv-integration.md)               | Installation, Home takeover, diagnostics and recovery             |
| [Compatibility](compatibility.md)                 | Clearly attributed upstream reports and fork verification status  |
| [Rendering](rendering.md)                         | Fonts, vectors, artwork, partial redraw and profiling             |
| [Releases](releases.md)                           | Changesets, CI gates, packaging and publication                   |
| [Licensing](licensing.md)                         | Copyright preservation and distributed notices                    |
| [Assets](assets.md)                               | Per-asset provenance, licenses and derived formats                |
| [Media](media/README.md)                          | Inherited screenshots and demo attribution                        |

Upstream project provenance and complete notice locations are in
[NOTICE.md](../NOTICE.md).

- [Control center and header](control-center.md): quick controls, value sliders and clock/avatar layout.

- [Liquid Glass](liquid-glass.md): optional native material and rendering budget.
- [CI duration](ci-performance.md): measured bottlenecks and proposed improvements.
