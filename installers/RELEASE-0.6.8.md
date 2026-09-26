# Saldonaut 0.6.8

## Highlights

- Adds a skippable, four-step welcome tour with visual previews for aggregation, local data processing, imports and the main financial views. The tour can be reopened from Help.
- Makes bank, portfolio and tax imports easier to discover with a shared clickable drag-and-drop design and visible file selection actions.
- Separates tax-return import from tax-year management. Tax history, editing and manual annual entry now live in the dedicated Taxes area.
- Refines import guidance, navigation, responsive layouts, keyboard focus and German, English, French and Italian translations.

## Windows download

Download `Saldonaut_0.6.8_x64-setup.exe` and its SHA-256 checksum from this release.
The installer is unsigned, so Windows may display a security warning.
Create an encrypted backup before updating or using real data.

## Validation

- 93 frontend tests passed.
- 158 Rust tests passed; one test was intentionally ignored.
- Frontend production build passed.
- Windows x64 release build and NSIS packaging completed successfully with native Strawberry Perl.
- License notices regenerated; personal build-path check passed.
- Installer product name Saldonaut, version 0.6.8, SHA-256 checksum `a3599931a8e266c6a6546906245c47a0963d92ecccfd7dab3530457883bbd200` and unsigned Authenticode status verified.
- No clean-machine installation test, full packaged-app regression test or complete manual visual/keyboard regression was performed.
- This release provides Windows x64 only. The previously published macOS 0.6.5 download remains available; macOS 0.6.8 has not been built or tested.
