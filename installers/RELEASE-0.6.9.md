# Saldonaut 0.6.9

## Highlights

- Stops automatically reclassifying existing accounts when a database is opened. No personal-data migration is shipped with the application.
- Removes two destructive anonymization commands from the application API and limits external URL opening to TradingView.
- Enforces a 2 GiB backup-restore limit with bounded copying and cleanup of incomplete profiles.
- Avoids displaying full local paths in import feedback and refines the reviewed account, transaction, position and tax interfaces.
- Updates the architecture and design guidance, translations and related tests.

## Windows download

Download `Saldonaut_0.6.9_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned, so Windows may display a security warning.
Create an encrypted backup before updating or using real data.

## Validation

- 96 frontend tests passed.
- 160 Rust tests passed; one test was intentionally ignored.
- Frontend production build and Rust Clippy checks passed.
- Windows x64 release build and NSIS packaging completed successfully with native Strawberry Perl.
- Personal build-path check passed.
- Installer product name Saldonaut, version 0.6.9, SHA-256 checksum `d7d2c91b58c5d6e1dbf3b774b0ee00bd8e5d7b6af3234b9b1103fb4582dd4206` and unsigned Authenticode status verified.
- No clean-machine installation test or full packaged-app regression test was performed.
- This release provides Windows x64 only. The previously published macOS 0.6.5 download remains available; macOS 0.6.9 has not been built or tested.
