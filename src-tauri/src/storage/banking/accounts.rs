//! Kapselt SQL-Zugriffe auf Konten und Anbieter.
use crate::domain::banking::accounts::{
    clean_optional, default_include_in_net_worth, validate_account,
};
use crate::storage::banking::models::{
    CreateAccountRequest, CreateInstitutionRequest, ManagedAccount, ManagedInstitution,
    UpdateAccountRequest, UpdateInstitutionRequest,
};
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use chrono::Utc;
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};

pub(crate) fn list_accounts(storage: &Storage) -> Result<Vec<ManagedAccount>, String> {
    accounts_from(storage)
}

pub(crate) fn list_institutions(storage: &Storage) -> Result<Vec<ManagedInstitution>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, provider_key, institution_type, logo_data_url
             FROM institutions ORDER BY name COLLATE NOCASE",
        )
        .map_err(db_error)?;
    let institutions = statement
        .query_map([], |row| {
            Ok(ManagedInstitution {
                id: row.get(0)?,
                name: row.get(1)?,
                provider_key: row.get(2)?,
                institution_type: row.get(3)?,
                logo_data_url: row.get(4)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(institutions)
}

pub(crate) fn create_institution(
    storage: &Storage,
    request: CreateInstitutionRequest,
) -> Result<i64, String> {
    validate_institution(&request.name, &request.institution_type)?;
    let connection = storage.connect().map_err(db_error)?;
    let duplicate: Option<i64> = connection
        .query_row(
            "SELECT id FROM institutions WHERE lower(name)=lower(?1)",
            [request.name.trim()],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?;
    if duplicate.is_some() {
        return Err("Eine Bank oder ein Anbieter mit diesem Namen ist bereits vorhanden.".into());
    }
    let logo_data_url = normalize_institution_logo(request.logo_data_url)?;
    let provider_key = unique_provider_key(&connection, &request.name)?;
    connection
        .execute(
            "INSERT INTO institutions(provider_key, name, institution_type, logo_data_url, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![provider_key, request.name.trim(), request.institution_type, logo_data_url, Utc::now().to_rfc3339()],
        )
        .map_err(db_error)?;
    Ok(connection.last_insert_rowid())
}

pub(crate) fn create_account(
    storage: &Storage,
    request: CreateAccountRequest,
) -> Result<i64, String> {
    validate_account(
        &request.account_name,
        &request.currency,
        &request.account_type,
    )?;
    let connection = storage.connect().map_err(db_error)?;
    let now = Utc::now().to_rfc3339();
    let institution_id: i64 = if let Some(institution_id) = request.institution_id {
        connection
            .query_row(
                "SELECT id FROM institutions WHERE id=?1",
                [institution_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    "Der Anbieter wurde nicht gefunden.".to_string()
                }
                other => db_error(other),
            })?
    } else {
        validate_institution(&request.institution_name, &request.institution_type)?;
        let provider_key = unique_provider_key(&connection, &request.institution_name)?;
        connection.execute(
            "INSERT OR IGNORE INTO institutions(provider_key, name, institution_type, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![provider_key, request.institution_name.trim(), request.institution_type, now],
        ).map_err(db_error)?;
        connection
            .query_row(
                "SELECT id FROM institutions WHERE provider_key = ?1",
                [&provider_key],
                |row| row.get(0),
            )
            .map_err(db_error)?
    };
    let include_in_net_worth = default_include_in_net_worth(&request.account_type);
    connection.execute(
        "INSERT INTO accounts(institution_id, name, account_type, currency, created_at, external_reference, include_in_net_worth) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![institution_id, request.account_name.trim(), request.account_type, request.currency.trim().to_uppercase(), now, clean_optional(request.external_reference), include_in_net_worth],
    ).map_err(|error| match error {
        rusqlite::Error::SqliteFailure(_, _) => "Dieses Konto ist bei diesem Anbieter bereits vorhanden.".to_string(),
        other => db_error(other),
    })?;
    Ok(connection.last_insert_rowid())
}

pub(crate) fn update_account(
    storage: &Storage,
    request: UpdateAccountRequest,
) -> Result<(), String> {
    validate_account(&request.name, &request.currency, &request.account_type)?;
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    let changed = transaction.execute(
        "UPDATE accounts SET name = ?1, account_type = ?2, currency = ?3, external_reference = ?4, is_active = ?5, include_in_net_worth = ?6 WHERE id = ?7",
        params![request.name.trim(), request.account_type, request.currency.trim().to_uppercase(), clean_optional(request.external_reference), request.is_active, request.include_in_net_worth, request.id],
    ).map_err(db_error)?;
    if changed == 0 {
        return Err("Das Konto wurde nicht gefunden.".to_string());
    }
    transaction.commit().map_err(db_error)
}

pub(crate) fn update_institution(
    storage: &Storage,
    request: UpdateInstitutionRequest,
) -> Result<(), String> {
    validate_institution(&request.name, &request.institution_type)?;
    let connection = storage.connect().map_err(db_error)?;
    let duplicate: Option<i64> = connection
        .query_row(
            "SELECT id FROM institutions WHERE lower(name)=lower(?1) AND id<>?2",
            params![request.name.trim(), request.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?;
    if duplicate.is_some() {
        return Err(
            "Eine andere Bank oder ein anderer Anbieter verwendet diesen Namen bereits.".into(),
        );
    }
    let changed = connection
        .execute(
            "UPDATE institutions SET name=?1, institution_type=?2 WHERE id=?3",
            params![request.name.trim(), request.institution_type, request.id],
        )
        .map_err(db_error)?;
    if changed == 0 {
        Err("Der Anbieter wurde nicht gefunden.".into())
    } else {
        Ok(())
    }
}

fn validate_institution(name: &str, institution_type: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err("Bitte einen Banknamen mit 1 bis 120 Zeichen eingeben.".into());
    }
    if !matches!(
        institution_type,
        "bank" | "insurance" | "broker" | "pension" | "self_custody"
    ) {
        return Err("Bitte einen gültigen Anbietertyp auswählen.".into());
    }
    Ok(())
}

pub(crate) fn delete_account(storage: &Storage, account_id: i64) -> Result<(), String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    let exists = transaction
        .query_row("SELECT 1 FROM accounts WHERE id = ?1", [account_id], |_| {
            Ok(())
        })
        .optional()
        .map_err(db_error)?;
    if exists.is_none() {
        return Err("Das Konto wurde nicht gefunden.".to_string());
    }
    let related_data: i64 = transaction
        .query_row(
            "SELECT (SELECT COUNT(*) FROM import_runs WHERE account_id = ?1)
                  + (SELECT COUNT(*) FROM transactions WHERE account_id = ?1)
                  + (SELECT COUNT(*) FROM balance_snapshots WHERE account_id = ?1)
                  + (SELECT COUNT(*) FROM portfolio_positions WHERE account_id = ?1)",
            [account_id],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if related_data > 0 {
        return Err("Dieses Konto enthält Importdaten und kann deshalb nicht gelöscht werden. Du kannst es stattdessen archivieren.".to_string());
    }
    transaction
        .execute("DELETE FROM accounts WHERE id = ?1", [account_id])
        .map_err(db_error)?;
    transaction.commit().map_err(db_error)
}

pub(crate) fn set_institution_logo(
    storage: &Storage,
    institution_id: i64,
    data_url: Option<String>,
) -> Result<(), String> {
    let value = normalize_institution_logo(data_url)?;
    let connection = storage.connect().map_err(db_error)?;
    let changed = connection
        .execute(
            "UPDATE institutions SET logo_data_url = ?1 WHERE id = ?2",
            params![value, institution_id],
        )
        .map_err(db_error)?;
    if changed == 0 {
        Err("Der Anbieter wurde nicht gefunden.".to_string())
    } else {
        Ok(())
    }
}

fn normalize_institution_logo(data_url: Option<String>) -> Result<Option<String>, String> {
    let value = data_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(value) = &value {
        let allowed = [
            "data:image/png;base64,",
            "data:image/jpeg;base64,",
            "data:image/webp;base64,",
            "data:image/svg+xml;base64,",
        ];
        if !allowed.iter().any(|prefix| value.starts_with(prefix)) {
            return Err("Bitte ein Logo im Format PNG, JPEG, WebP oder SVG auswählen.".to_string());
        }
        if value.len() > 2_800_000 {
            return Err("Das Logo darf maximal 2 MB gross sein.".to_string());
        }
    }
    Ok(value)
}

pub(crate) fn accounts_from(storage: &Storage) -> Result<Vec<ManagedAccount>, String> {
    let connection = storage.connect().map_err(db_error)?;
    accounts_on(&connection)
}

pub(crate) fn accounts_on(connection: &Connection) -> Result<Vec<ManagedAccount>, String> {
    let mut statement = connection.prepare(
        "SELECT a.id, i.id, i.name, i.provider_key, i.institution_type, a.name, a.account_type,
                a.currency, a.external_reference, a.is_active, a.include_in_net_worth,
                CASE WHEN a.account_type IN ('portfolio','manual_asset','pillar3a') THEN (SELECT SUM(d.value_minor) FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id AND date(p.holding_start_date)<=date('now','localtime') AND (p.holding_end_date IS NULL OR date(p.holding_end_date)>=date('now','localtime'))) WHEN bs.id IS NULL THEN NULL ELSE bs.amount_minor + COALESCE((
                  SELECT SUM(t.amount_minor) FROM transactions t
                  WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                    AND date(t.booking_date) <= date('now', 'localtime')
                    AND NOT EXISTS (SELECT 1 FROM ignored_duplicate_transactions ignored WHERE ignored.transaction_id=t.id)
                ), 0) END,
                CASE WHEN a.account_type IN ('portfolio','manual_asset','pillar3a') THEN (SELECT MAX(d.valuation_date) FROM portfolio_positions p JOIN daily_valuations d ON d.position_id=p.id WHERE p.account_id=a.id AND date(d.valuation_date)<=date('now','localtime')) WHEN bs.id IS NULL THEN NULL ELSE COALESCE((
                  SELECT MAX(t.booking_date) FROM transactions t
                  WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                    AND date(t.booking_date) <= date('now', 'localtime')
                    AND NOT EXISTS (SELECT 1 FROM ignored_duplicate_transactions ignored WHERE ignored.transaction_id=t.id)
                ), bs.balance_date) END,
                CASE WHEN a.account_type IN ('portfolio','manual_asset','pillar3a') THEN COALESCE((SELECT d.currency FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id LIMIT 1),a.currency) ELSE a.currency END,
                (SELECT COUNT(*) FROM import_runs ir WHERE ir.account_id = a.id OR EXISTS(SELECT 1 FROM balance_snapshots s WHERE s.import_id = ir.id AND s.account_id = a.id)),
                (SELECT COUNT(*) FROM portfolio_positions p WHERE p.account_id = a.id),
                NULL, NULL, NULL, NULL, i.logo_data_url
         FROM accounts a JOIN institutions i ON i.id = a.institution_id
         LEFT JOIN balance_snapshots bs ON bs.id = (SELECT latest.id FROM balance_snapshots latest WHERE latest.account_id = a.id AND date(latest.balance_date) <= date('now', 'localtime') ORDER BY latest.balance_date DESC, latest.id DESC LIMIT 1)
         ORDER BY i.name, a.is_active DESC, a.name"
    ).map_err(db_error)?;
    let accounts = statement
        .query_map([], |row| {
            Ok(ManagedAccount {
                id: row.get(0)?,
                institution_id: row.get(1)?,
                provider: row.get(2)?,
                provider_key: row.get(3)?,
                institution_type: row.get(4)?,
                name: row.get(5)?,
                account_type: row.get(6)?,
                currency: row.get(7)?,
                external_reference: row.get(8)?,
                is_active: row.get(9)?,
                include_in_net_worth: row.get(10)?,
                balance_minor: row.get(11)?,
                balance_date: row.get(12)?,
                balance_currency: row.get(13)?,
                import_count: row.get(14)?,
                manual_valuation_count: row.get(15)?,
                manual_quantity: row.get(16)?,
                manual_unit_price_minor: row.get(17)?,
                manual_quote_currency: row.get(18)?,
                manual_exchange_rate: row.get(19)?,
                logo_data_url: row.get(20)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(accounts)
}

pub(crate) fn unique_provider_key(connection: &Connection, name: &str) -> Result<String, String> {
    if let Some(key) = connection
        .query_row(
            "SELECT provider_key FROM institutions WHERE lower(name) = lower(?1)",
            [name.trim()],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?
    {
        return Ok(key);
    }
    let generated = name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let normalized = generated
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if normalized.is_empty() {
        "anbieter"
    } else {
        &normalized
    };
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while connection
        .query_row(
            "SELECT 1 FROM institutions WHERE provider_key = ?1",
            [&candidate],
            |_| Ok(()),
        )
        .optional()
        .map_err(db_error)?
        .is_some()
    {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}
