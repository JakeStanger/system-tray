# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.7.0] - 2025-02-08
### :sparkles: New Features
- [`20de4bd`](https://github.com/JakeStanger/system-tray/commit/20de4bd907f1fe72d30e0e140e88e52d684814dc) - add dbusmenu `aboutToShow` mapping to client *(PR [#12](https://github.com/JakeStanger/system-tray/pull/12) by [@ogios](https://github.com/ogios))*
  - :arrow_lower_right: *addresses issue [#8](https://github.com/JakeStanger/system-tray/issues/8) opened by [@ogios](https://github.com/ogios)*

### :bug: Bug Fixes
- [`a3d8421`](https://github.com/JakeStanger/system-tray/commit/a3d842136357b5a0a3976e46ed83d803797e768f) - remove items from internal state when receiving remove event *(PR [#17](https://github.com/JakeStanger/system-tray/pull/17) by [@Levizor](https://github.com/Levizor))*

### :recycle: Refactors
- [`d69e0e4`](https://github.com/JakeStanger/system-tray/commit/d69e0e4aa9d9b773377e8f1f7d345663decbd7d7) - **item**: use `transpose` for flipping option/result *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6d727c6`](https://github.com/JakeStanger/system-tray/commit/6d727c6dd7e174e374f6ef37ca4998c1348bc558) - upgrade to zbus v5 *(PR [#14](https://github.com/JakeStanger/system-tray/pull/14) by [@ogios](https://github.com/ogios))*
  - :arrow_lower_right: *addresses issue [#13](https://github.com/JakeStanger/system-tray/issues/13) opened by [@ogios](https://github.com/ogios)*
- [`4953f65`](https://github.com/JakeStanger/system-tray/commit/4953f65c6f7d5612eb21262622e3b8fafae36f58) - avoid using re-exported `futures_util` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`47b8f5d`](https://github.com/JakeStanger/system-tray/commit/47b8f5da494395cbed5245d6f57d68ae93fbc86f) - suppress unused warning *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f4572f4`](https://github.com/JakeStanger/system-tray/commit/f4572f439d78f28b5391b2e75704950b574b6b0e) - cleanup return type *(commit by [@JakeStanger](https://github.com/JakeStanger))*

[v0.7.0]: https://github.com/JakeStanger/system-tray/compare/v0.6.0...v0.7.0
