//! Wählt einen Positionsparser aus der Provider-Registry und normalisiert Kontoreferenzen am Import-Rand.

use crate::domain::securities::position_snapshots::PositionSnapshot;
use std::path::Path;

pub(crate) fn parse(path: &Path) -> Result<Option<PositionSnapshot>, String> {
    for provider in super::providers::position_providers() {
        if let Some(snapshot) = provider.parse_positions(path)? {
            return Ok(Some(snapshot));
        }
    }
    Ok(None)
}

pub(crate) fn account_reference(provider: &str, value: &str) -> String {
    super::providers::by_id(provider).map_or_else(
        || crate::domain::securities::position_snapshots::normalize_reference(value),
        |parser| parser.position_reference(value),
    )
}
