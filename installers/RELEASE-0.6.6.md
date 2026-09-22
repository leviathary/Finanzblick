# Saldonaut 0.6.6

## Highlights

- Adds **Accounts & portfolios**, a read-only view separate from account management, with account balances, bookings, portfolio positions and value/price charts.
- Orders the main navigation from overview to detail: Overview, Net worth, Accounts & portfolios, Transactions.
- Supports adding and managing positions in portfolio accounts just as in manually managed investment accounts.
- Adds provider-independent, dated position snapshot imports. Swissquote is the first input provider; reconciliation, date validation and persistence are shared.
- Preserves snapshot effective dates and distinguishes full and partial holdings. No synthetic purchase or sale transactions are created.
- Refines back navigation and spacing in position details, with German, English, French and Italian translations.
- Updates the README with a demo screenshot.

## Windows download

Download `Saldonaut_0.6.6_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned, so Windows may display a security warning.
Create an encrypted backup before updating or using real data.

## Validation

- 86 frontend tests passed.
- 158 Rust tests passed; one test was intentionally ignored.
- Frontend production build passed.
- Browser checks covered light/dark appearance, narrow layouts, keyboard navigation, charts and read-only detail views using synthetic data.
- Windows x64 release build and NSIS packaging completed successfully with native Strawberry Perl.
- License notices regenerated; personal build-path check passed.
- Installer product name Saldonaut, version 0.6.6, SHA-256 checksum and unsigned Authenticode status verified.
- No clean-machine installation test or full packaged-app regression test was performed. Browser checks do not replace those tests.
- This release provides Windows x64 only. The previously published macOS 0.6.5 download remains available; macOS 0.6.6 has not been built or tested.
