# Saldonaut 0.6.4

## Highlights

- The application, installer, window title and documentation now use the Saldonaut name and visual identity.
- The login screen and application icons use the new Saldonaut artwork.
- The README includes the new Saldonaut hero image and current Windows download links.
- Existing local financial profiles remain compatible; the application identifier and database location are unchanged.

## Windows download

Download `Saldonaut_0.6.4_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned, so Windows may display a security warning.
Create an encrypted backup before updating or using real data.

## Validation

- 78 frontend tests passed.
- 140 Rust tests passed; one test was intentionally ignored.
- Windows x64 release build completed, license inventory regenerated and personal build-path check passed.
- Installer product name, file version and SHA-256 checksum were verified.
- The login screen was visually reviewed in light and dark mode during the rename work.
- A clean-machine installation test, a final packaged keyboard regression pass and a macOS build were not part of this release verification.
