# Packaging

Installers are built with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager), configured in `[package.metadata.packager]` of [ui/Cargo.toml](../ui/Cargo.toml). The app's version is the `ui` crate's version.

| System | Formats | Built on |
|---|---|---|
| Linux | `.deb`, `.AppImage` | ubuntu-22.04, so the binary also runs on older glibc versions |
| Windows | NSIS `-setup.exe`, WiX `.msi`, portable `.zip` | windows-latest |
| macOS | `.dmg` (Apple silicon) | macos-latest |

## Building locally
```sh
cargo install cargo-packager --locked
cd ui
cargo packager --release --formats deb,appimage   # Linux; nsis,wix on Windows; dmg on macOS
```
The packages land in `target/packages/`.

The executable is `GeneticAlgo` (`GeneticAlgo.exe` on Windows), set by `[[bin]]` in `ui/Cargo.toml`: Cargo allows no spaces in it, so the display name "Genetic Algo" comes from the packages and `ui/src/identity.rs`. The package files are named after it too, such as `GeneticAlgo_<version>_amd64.deb`.

## Releasing
1. Bump `version` in `ui/Cargo.toml`, merge it to `main`.
2. Tag the merge commit `v<version>` and push the tag. The release workflow checks that the two match.
3. [.github/workflows/release.yml](../.github/workflows/release.yml) builds every package, writes `checksums.txt`, attests the build provenance and publishes a GitHub release.

A pull request that touches packaging runs the same builds without publishing.

## Desktop identity
The desktop matches a running window to its launcher and icon by the app ID, `io.github.riccardogabellone.GeneticAlgo` (see [ui/src/identity.rs](../ui/src/identity.rs)). cargo-packager names its generated desktop entry and icons after the binary instead, so that entry is turned off and the packages ship the files in [linux/](linux/), named after the ID:
- `<ID>.desktop`, the launcher entry; a test checks it agrees with `identity.rs`;
- `<ID>.metainfo.xml`, the AppStream description used by software centres;
- the icons from [assets/icons/](../assets/icons/), in `hicolor/<size>/apps/<ID>.png`, plus the SVG master as `scalable`.

The AppImage tooling looks for the desktop entry under the binary's name, so the AppImage gets the same file under that name.

## Not signed yet
The packages are not code-signed, so on first launch:
- **macOS** says the app is damaged or from an unidentified developer. Run `xattr -dr com.apple.quarantine "/Applications/Genetic Algo.app"`, or allow it under System Settings, Privacy & Security.
- **Windows** SmartScreen warns about an unrecognised app: choose "More info", then "Run anyway".

Signing needs an Apple Developer ID (notarisation) and a Windows code-signing certificate, for example Azure Artifact Signing; cargo-packager has `macos.signing-identity` and `windows.sign-command` for them.
