# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.8.5] - 2025-12-28
### :sparkles: New Features
- [`608b24f`](https://github.com/JakeStanger/system-tray/commit/608b24f215fb555f59fe57f675cddbffbff49ba7) - implement Serialize for various structs *(PR [#27](https://github.com/JakeStanger/system-tray/pull/27) by [@maxdexh](https://github.com/maxdexh))*


## [v0.8.4] - 2025-10-12
### :sparkles: New Features
- [`02740b1`](https://github.com/JakeStanger/system-tray/commit/02740b1f9c730243cafb0b87e2e344014e84c547) - derive `Eq`, `PartialEq`, `Hash` where it makes sense *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`8c19f78`](https://github.com/JakeStanger/system-tray/commit/8c19f78b7a1fde876a2afd52a99cfe84f73e5f4d) - **readme**: update libdbusmenu-gtk3 notes *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.8.3] - 2025-09-20
### :bug: Bug Fixes
- [`fdfb100`](https://github.com/JakeStanger/system-tray/commit/fdfb100ccc9b237b5ebc0fd6b4d3e6fe150804eb) - **client**: debounce menu layout updates *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`f7a1551`](https://github.com/JakeStanger/system-tray/commit/f7a15519c900f773e4ed052a847e9a0ae1bf389e) - **menu**: avoid logging full icon data in debug impl *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dfaea12`](https://github.com/JakeStanger/system-tray/commit/dfaea12c8bafa2599dee0c2e31f1ad033ce53cae) - **client**: tidy code *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.8.1] - 2025-06-27
### :bug: Bug Fixes
- [`cf2d0b3`](https://github.com/JakeStanger/system-tray/commit/cf2d0b36904cf5fff028f821fbe0c6680761c2f7) - take in account of InvalidArgs caused by no property *(commit by [@ogios](https://github.com/ogios))*


## [v0.8.0] - 2025-06-25
### :sparkles: New Features
- [`e107d45`](https://github.com/JakeStanger/system-tray/commit/e107d45a97c3c5e7d8d459c0d00e172132b19d2b) - make `Client::items` optional behind feature flag *(PR [#18](https://github.com/JakeStanger/system-tray/pull/18) by [@ogios](https://github.com/ogios))*

### :bug: Bug Fixes
- [`44c5547`](https://github.com/JakeStanger/system-tray/commit/44c5547bc52e76534d4262e9d63abe3404af392d) - NewIcon update event send both icon_name and icon_pixmap *(commit by [@ogios](https://github.com/ogios))*
- [`94719f3`](https://github.com/JakeStanger/system-tray/commit/94719f3d975e90105074a4f8d690e8586693e2b2) - update tray menu before sending MenuDiff event *(PR [#21](https://github.com/JakeStanger/system-tray/pull/21) by [@Levizor](https://github.com/Levizor))*
  - :arrow_lower_right: *fixes issue [#20](https://github.com/JakeStanger/system-tray/issues/20) opened by [@Levizor](https://github.com/Levizor)*
- [`2cf941f`](https://github.com/JakeStanger/system-tray/commit/2cf941fc8e4ee34cb66a833ba477f94496eb6db6) - NewIcon update event send both icon_name and icon_pixmap *(PR [#22](https://github.com/JakeStanger/system-tray/pull/22) by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`8da8443`](https://github.com/JakeStanger/system-tray/commit/8da8443fa8e82c5c7c7da7fc3de376e03b10dca1) - fix clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*


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
[v0.8.0]: https://github.com/JakeStanger/system-tray/compare/v0.7.0...v0.8.0
[v0.8.1]: https://github.com/JakeStanger/system-tray/compare/v0.8.0...v0.8.1
[v0.8.3]: https://github.com/JakeStanger/system-tray/compare/v0.8.2...v0.8.3
[v0.8.4]: https://github.com/JakeStanger/system-tray/compare/v0.8.3...v0.8.4
[v0.8.5]: https://github.com/JakeStanger/system-tray/compare/v0.8.4...v0.8.5
