# Saldonaut 0.6.10

## Highlights

- Adds a generic local PDF field-mapping workflow with reusable profiles for previously unknown bank statements.
- Moves the tested UBS and PostFinance account-statement layouts and the UBS credit-card statement to bundled declarative profiles.
- Lists bundled provider profiles and bank standards directly in the import view, including MT940, camt.053, camt.054 and Swissquote position statements.
- Improves duplicate review, running-balance matching, account-reference normalization and PDF import controls.
- Removes the synthetic UBS statement fixture that did not represent a real bank export.

## Windows download

Download `Saldonaut_0.6.10_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned, so Windows may display a security warning.
Create an encrypted backup before updating or using real data.

## Validation

- 102 frontend tests passed.
- 175 Rust tests passed; one test was intentionally ignored.
- Frontend production build and Rust Clippy checks passed.
- Windows x64 release build and NSIS packaging completed successfully with native Strawberry Perl.
- Personal build-path check passed.
- Installer product name Saldonaut, version 0.6.10, SHA-256 checksum `ff3b8581ca1c0de607cac665c695557a133e986ac2bc0bdec5423d8aa45629fe` and unsigned Authenticode status verified.
- No clean-machine installation test, visual regression pass or full packaged-app regression test was performed.
- This release provides Windows x64 only. The previously published macOS 0.6.5 download remains available; macOS 0.6.10 has not been built or tested.
