# Finanzblick 0.6.0

## Highlights

- More consistent, compact navigation and contextual action menus for accounts, categories and transactions.
- Interactive transaction balance chart with YTD as the default period; bank-account balances are separated from investment accounts.
- Donut charts for category spending and wealth by provider, plus multi-select bank and account filters.
- Inline category editing and a clearer, unified income-and-expense analysis area.
- A unified import workspace with per-file inline previews and preserved duplicate checks.
- Simplified profile management and separately encrypted anonymized copies that leave the original profile unchanged.

## Windows download

Download `Finanzblick_0.6.0_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned; Windows may show a security warning.
Back up your financial profile before updating.

## Validation

- 55 frontend tests passed.
- 118 Rust tests passed; one test intentionally ignored.
- Windows x64 release build completed, license inventory regenerated and personal build-path check passed.
- Installer version metadata and SHA-256 checksum verified.
- UI checks covered light/dark appearance, narrow layouts and keyboard interactions.
- The project maintainer successfully tested the Windows installer and confirmed the result on 2026-09-20.
- Testing on a clean Windows machine was not separately confirmed. A macOS build is not part of this release verification.
