# Saldonaut 0.6.11

## Highlights

- Imports Migros Bank pension-account statements and regular account statements
  from PDF, including account references, bookings and opening/closing balances.
- Normalizes whitespace and invisible copy/paste separators in IBANs and account
  references so imported statements match manually maintained accounts reliably.
- Groups the account overview into bank accounts, credit cards, investments and
  pension accounts, and reveals newly opened account forms by scrolling and focusing.
- Displays imported balances and bookings for cash-only pillar 3a accounts while
  continuing to use position valuations once positions exist.
- Keeps all committed importer and matching tests on synthetic account data.

## Windows download

Download `Saldonaut_0.6.11_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned, so Windows may display a security warning.
Create an encrypted backup before updating or using real data.

## Validation

- 106 frontend tests passed.
- 181 Rust tests passed; one test was intentionally ignored.
- Frontend production build and Rust Clippy checks passed.
- Windows x64 release build and NSIS packaging completed successfully with native
  Strawberry Perl in a fresh neutral target directory.
- Personal build-path check passed.
- Installer product name Saldonaut, version 0.6.11, SHA-256 checksum
  `f472a9c8369ef836515a96bca7f78e64d9c769c28a90415fd6fc5483603af5b2` and
  unsigned Authenticode status verified.
- No clean-machine installation test or full packaged-app regression test was
  performed. The account-detail mode switch was not manually checked visually or
  with the keyboard in the packaged app.
- This release provides Windows x64 only. macOS 0.6.11 has not been built or tested.
