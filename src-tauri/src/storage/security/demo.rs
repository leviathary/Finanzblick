use super::*;
use chrono::{Datelike, Duration as Days, Local, NaiveDate};
use rusqlite::params;

const PASSWORD: &str = "demo1234";

pub(super) fn is_reusable_demo(path: &Path, name: &str) -> bool {
    name.strip_prefix("Demo ")
        .and_then(|number| number.parse::<u32>().ok())
        .is_some()
        && open(path, PASSWORD, false).map(|db| is_demo(&db)).unwrap_or(false)
}

pub(crate) fn is_demo(db: &Connection) -> bool {
    db.query_row(
        "SELECT version FROM finanzblick_demo WHERE id=1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .is_ok()
}

#[tauri::command]
pub fn demo_status(storage: State<'_, Storage>) -> Result<bool, String> {
    let db = storage.connect().map_err(|_| LOCKED)?;
    Ok(is_demo(&db))
}

#[tauri::command]
pub async fn create_demo_database(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .create_demo(Local::now().date_naive())
    })
    .await
    .map_err(|_| "Demo konnte nicht erstellt werden.".to_string())?
}

impl Storage {
    fn create_demo(&self, today: NaiveDate) -> Result<(), String> {
        let mut session = self.session.write().map_err(|_| LOCKED)?;
        let directory = self.path.parent().ok_or("Speicherort fehlt.")?;
        let mut candidates = Vec::new();
        for entry in
            fs::read_dir(directory).map_err(|_| "Finanzprofile konnten nicht gelesen werden.")?
        {
            let entry = entry.map_err(|_| "Finanzprofile konnten nicht gelesen werden.")?;
            let filename = entry.file_name().to_string_lossy().into_owned();
            if let Some(name) = filename.strip_suffix(".vault.sqlite3") {
                if let Some(number) = name
                    .strip_prefix("Demo ")
                    .and_then(|value| value.parse::<u32>().ok())
                {
                    candidates.push((
                        name != session.database_id,
                        number,
                        name.to_string(),
                        entry.path(),
                    ));
                }
            }
        }
        candidates.sort();
        for (_, _, name, path) in candidates {
            let db = open(&path, PASSWORD, false).map_err(|_| {
                format!("{name} konnte nicht geöffnet werden. Bitte dieses Finanzprofil direkt auswählen und mit seinem Passwort entsperren.")
            })?;
            if !is_demo(&db) {
                continue;
            }
            initialize_schema(&db).map_err(|_| "Demo konnte nicht geöffnet werden.")?;
            let settings = read_settings(&db)?;
            self.persist_database_choice(&name)?;
            session.clear();
            session.database_id = name;
            session.password = Some(Zeroizing::new(PASSWORD.into()));
            session.activity = Some(SystemTime::now());
            session.retry_after = None;
            session.settings = settings;
            return Ok(());
        }
        let name = (1..10000)
            .map(|number| format!("Demo {number}"))
            .find(|name| self.available_destination(name).is_ok())
            .ok_or("Kein freier Profilname.")?;
        let destination = self.available_destination(&name)?;
        let stage = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| "Demo konnte nicht erstellt werden.")?;
        {
            let mut db = open(stage.path(), PASSWORD, true)
                .map_err(|_| "Demo konnte nicht erstellt werden.")?;
            seed(&mut db, today)
                .map_err(|error| format!("Demo konnte nicht erstellt werden: {error}"))?;
        }
        stage
            .as_file()
            .sync_all()
            .map_err(|_| "Demo konnte nicht gespeichert werden.")?;
        stage
            .persist_noclobber(destination)
            .map_err(|_| "Demo konnte nicht gespeichert werden.")?;
        self.persist_database_choice(&name)?;
        session.clear();
        session.database_id = name;
        session.password = Some(Zeroizing::new(PASSWORD.into()));
        session.activity = Some(SystemTime::now());
        session.settings = AppSettings::default();
        Ok(())
    }
}

struct Holding {
    name: &'static str,
    symbol: &'static str,
    kind: &'static str,
    currency: &'static str,
    start_month: i32,
    end_month: Option<i32>,
    units: i64,
    base: f64,
    growth: f64,
}

fn holdings() -> [Holding; 6] {
    [
        Holding {
            name: "Apple",
            symbol: "AAPL",
            kind: "stock",
            currency: "USD",
            start_month: 0,
            end_month: None,
            units: 80,
            base: 45.0,
            growth: 0.18,
        },
        Holding {
            name: "Nasdaq-100 ETF (Demo)",
            symbol: "DEMO-NDX",
            kind: "fund",
            currency: "USD",
            start_month: 8,
            end_month: None,
            units: 100,
            base: 90.0,
            growth: 0.12,
        },
        Holding {
            name: "Microsoft",
            symbol: "MSFT",
            kind: "stock",
            currency: "USD",
            start_month: 18,
            end_month: None,
            units: 35,
            base: 100.0,
            growth: 0.15,
        },
        Holding {
            name: "NVIDIA",
            symbol: "NVDA",
            kind: "stock",
            currency: "USD",
            start_month: 26,
            end_month: None,
            units: 120,
            base: 15.0,
            growth: 0.30,
        },
        Holding {
            name: "Nestlé",
            symbol: "NESN",
            kind: "stock",
            currency: "CHF",
            start_month: 38,
            end_month: None,
            units: 60,
            base: 85.0,
            growth: 0.025,
        },
        Holding {
            name: "Tesla (verkauft)",
            symbol: "TSLA",
            kind: "stock",
            currency: "USD",
            start_month: 44,
            end_month: Some(74),
            units: 30,
            base: 60.0,
            growth: 0.17,
        },
    ]
}

fn date_at(year: i32, month_offset: i32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(
        year + month_offset / 12,
        (month_offset % 12 + 1) as u32,
        day,
    )
    .unwrap()
}

fn price(holding: &Holding, elapsed: f64, index: usize) -> i64 {
    let cycle =
        1.0 + 0.065 * (elapsed / 47.0 + index as f64).sin() + 0.035 * (elapsed / 181.0).sin();
    let downturn = 1.0
        - 0.28 * (-((elapsed - 800.0) / 65.0).powi(2)).exp()
        - 0.22 * (-((elapsed - 1700.0) / 180.0).powi(2)).exp();
    (holding.base * (holding.growth * elapsed / 365.25).exp() * cycle * downturn * 100.0).round()
        as i64
}

fn booking(
    db: &Connection,
    account: i64,
    import_id: i64,
    day: NaiveDate,
    description: &str,
    amount: i64,
    balance: &mut i64,
    row: &mut i64,
    category: &str,
) -> rusqlite::Result<()> {
    *balance += amount;
    *row += 1;
    db.execute("INSERT INTO transactions(account_id,import_id,booking_date,value_date,description,amount_minor,balance_minor,currency,confidence,source_row,category_id,category_manual,category_source) VALUES(?1,?2,?3,?3,?4,?5,?6,'CHF',1,?7,(SELECT id FROM categories WHERE category_key=?8),1,'manual')",
        params![account, import_id, day.to_string(), description, amount, *balance, *row, category])?;
    Ok(())
}

fn seed(db: &mut Connection, today: NaiveDate) -> rusqlite::Result<()> {
    initialize_schema(db)?;
    let transaction = db.transaction()?;
    let first_year = today.year() - 8;
    let start = date_at(first_year, 0, 1);
    let stamp = today.to_string();
    transaction.execute_batch("CREATE TABLE finanzblick_demo(id INTEGER PRIMARY KEY,version INTEGER NOT NULL); INSERT INTO finanzblick_demo VALUES(1,1);")?;
    for (id, key, name) in [
        (1, "ubs", "UBS"),
        (2, "raiffeisen", "Raiffeisen"),
        (3, "swissquote", "Swissquote"),
    ] {
        transaction.execute("INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(?1,?2,?3,'bank',?4)", params![id,key,name,start.to_string()])?;
    }
    for (id, name, kind) in [
        (1, "Privatkonto – Alltag", "cash"),
        (2, "Sparkonto – Reserve", "savings"),
        (3, "Depot – Aktien & ETF", "manual_asset"),
    ] {
        transaction.execute("INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(?1,?1,?2,?3,'CHF',?4)",params![id,name,kind,start.to_string()])?;
    }
    transaction.execute("INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(4,1,'Mastercard – Alltag','credit_card','CHF',?1)", [start.to_string()])?;
    let positions = holdings();
    for (index, holding) in positions.iter().enumerate() {
        let id = index as i64 + 1;
        let opening = date_at(first_year, holding.start_month, 15);
        let closing = holding
            .end_month
            .map(|month| date_at(first_year, month, 15).to_string());
        transaction.execute(
            "INSERT INTO instruments(id,name,asset_type,created_at) VALUES(?1,?2,?3,?4)",
            params![id, holding.name, holding.kind, stamp],
        )?;
        transaction.execute("INSERT INTO instrument_listings(id,instrument_id,market_symbol,quote_currency,preferred_price_source) VALUES(?1,?1,?2,?3,'demo')",params![id,holding.symbol,holding.currency])?;
        transaction.execute("INSERT INTO instrument_identifiers(instrument_id,identifier_type,identifier) VALUES(?1,'ticker',?2)",params![id,holding.symbol])?;
        transaction.execute("INSERT INTO portfolio_positions(id,account_id,listing_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(?1,3,?1,?2,?3,?4,?5,?6,?6)",params![id,holding.name,holding.kind,opening.to_string(),closing,stamp])?;
        transaction.execute("INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(?1,?2,?3,0,'demo',?4)",params![id,opening.to_string(),holding.units,stamp])?;
        if holding.end_month.is_none() && opening + Days::days(730) <= today {
            transaction.execute("INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(?1,?2,?3,0,'demo',?4)",params![id,(opening+Days::days(730)).to_string(),holding.units+holding.units/2,stamp])?;
        }
        if let Some(closing) = &closing {
            transaction.execute("INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(?1,?2,0,0,'demo',?3)",params![id,closing,stamp])?;
        }
    }
    let mut cash = 5_000_000_i64;
    let mut savings = 1_500_000_i64;
    let mut card_balance = 0_i64;
    let mut import_card = 0;
    let mut row_card = 0;
    let mut import_cash = 0;
    let mut import_savings = 0;
    let mut row_cash = 0;
    let mut row_savings = 0;
    let mut day = start;
    while day <= today {
        let elapsed = (day - start).num_days() as f64;
        let month_index = (day.year() - first_year) * 12 + day.month0() as i32;
        if day.day() == 1 {
            for account in [1, 2, 4] {
                transaction.execute("INSERT INTO import_runs(account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json) VALUES(?1,?2,'DEMO',?3,?4,0,'[]')",params![account,format!("Demo-Auszug {} {:04}-{:02}",match account {1=>"UBS",2=>"Raiffeisen",_=>"UBS Mastercard"},day.year(),day.month()),format!("demo-{account}-{day}"),stamp])?;
                if account == 1 {
                    import_cash = transaction.last_insert_rowid();
                    row_cash = 0;
                } else if account == 2 {
                    import_savings = transaction.last_insert_rowid();
                    row_savings = 0;
                } else {
                    import_card = transaction.last_insert_rowid();
                    row_card = 0;
                }
            }
        }
        for (purchase_day, merchant, amount, category) in [
            (4, "Coop – Einkauf (Demo)", -8_900, "groceries"),
            (9, "Restaurant Seeblick (Demo)", -7_600, "restaurants"),
            (13, "SBB – Billette (Demo)", -4_200, "transport"),
            (17, "Digitec – Zubehör (Demo)", -12_900, "electronics"),
            (21, "Kino – Eintritt (Demo)", -3_600, "leisure"),
            (
                23,
                "Netflix – Abonnement (Demo)",
                -1_990,
                "digital_subscriptions",
            ),
            (26, "Apotheke – Einkauf (Demo)", -2_450, "health"),
        ] {
            if day.day() == purchase_day {
                let variation = if category == "digital_subscriptions" {
                    0
                } else {
                    month_index as i64 * 73 % 2000
                };
                booking(
                    &transaction,
                    4,
                    import_card,
                    day,
                    merchant,
                    amount - variation,
                    &mut card_balance,
                    &mut row_card,
                    category,
                )?;
            }
        }
        if day.day() == 24 && day.month() % 3 == 0 {
            booking(
                &transaction,
                4,
                import_card,
                day,
                "Digitec – Rückerstattung (Demo)",
                3_000,
                &mut card_balance,
                &mut row_card,
                "electronics",
            )?;
        }
        if day.day() == 28 && card_balance < 0 {
            let payment = -card_balance;
            booking(
                &transaction,
                1,
                import_cash,
                day,
                "UBS Kreditkartenabrechnung – Mastercard (Demo)",
                -payment,
                &mut cash,
                &mut row_cash,
                "other",
            )?;
            booking(
                &transaction,
                4,
                import_card,
                day,
                "LSV-ZAHLUNG – Rechnungsausgleich (Demo)",
                payment,
                &mut card_balance,
                &mut row_card,
                "other",
            )?;
        }
        if day.day() == 2 {
            booking(
                &transaction,
                1,
                import_cash,
                day,
                "Lohn – Beispiel AG",
                720_000 + (day.year() - first_year) as i64 * 20_000,
                &mut cash,
                &mut row_cash,
                "income",
            )?;
            booking(
                &transaction,
                1,
                import_cash,
                day,
                "Miete – Musterwohnung",
                -210_000,
                &mut cash,
                &mut row_cash,
                "housing",
            )?;
        }
        if day.day() == 5 {
            booking(
                &transaction,
                1,
                import_cash,
                day,
                "Übertrag auf eigenes Raiffeisen-Sparkonto",
                -80_000,
                &mut cash,
                &mut row_cash,
                "saving",
            )?;
            booking(
                &transaction,
                2,
                import_savings,
                day,
                "Übertrag vom eigenen UBS-Privatkonto",
                80_000,
                &mut savings,
                &mut row_savings,
                "saving",
            )?;
        }
        for (booking_day, description, amount, category) in [
            (7, "Krankenkasse – Demo", -38_000, "health"),
            (10, "Lebensmittel – Wocheneinkäufe", -65_000, "groceries"),
            (12, "Bahnabo – Demo", -18_000, "transport"),
            (18, "Internet und Mobilfunk", -9_000, "telecom"),
            (22, "Restaurants und Freizeit", -24_000, "restaurants"),
            (25, "Steuerrücklage – Zahlung", -60_000, "taxes"),
        ] {
            if day.day() == booking_day {
                let variation = if category == "groceries" || category == "restaurants" {
                    (month_index as i64 * 137) % 6000
                } else {
                    0
                };
                booking(
                    &transaction,
                    1,
                    import_cash,
                    day,
                    description,
                    amount - variation,
                    &mut cash,
                    &mut row_cash,
                    category,
                )?;
            }
        }
        if day.month() == 7 && day.day() == 20 {
            booking(
                &transaction,
                1,
                import_cash,
                day,
                "Sommerferien – Demo",
                -240_000,
                &mut cash,
                &mut row_cash,
                "travel",
            )?;
        }
        let fx = (0.96 - elapsed / 365.25 * 0.012 + 0.025 * (elapsed / 160.0).sin()) * 1_000_000.0;
        let fx_amount = fx.round() as i64;
        transaction.execute("INSERT INTO fx_rates(base_currency,quote_currency,rate_date,rate_amount,rate_scale,source,fetched_at) VALUES('USD','CHF',?1,?2,6,'demo',?3)",params![day.to_string(),fx_amount,stamp])?;
        let fx_id = transaction.last_insert_rowid();
        let mut portfolio = 0_i64;
        for (index, holding) in positions.iter().enumerate() {
            let id = index as i64 + 1;
            let opening = date_at(first_year, holding.start_month, 15);
            let closing = holding
                .end_month
                .map(|month| date_at(first_year, month, 15));
            if day < opening || closing.is_some_and(|end| day > end) {
                continue;
            }
            let added = holding.end_month.is_none() && day >= opening + Days::days(730);
            let units = holding.units + if added { holding.units / 2 } else { 0 };
            let price_minor = price(holding, elapsed, index);
            let rate = if holding.currency == "USD" {
                fx_amount as f64 / 1_000_000.0
            } else {
                1.0
            };
            let value = (units as f64 * price_minor as f64 * rate).round() as i64;
            transaction.execute("INSERT INTO instrument_prices(listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at) VALUES(?1,?2,'eod_close',?3,2,?4,'demo',?5)",params![id,day.to_string(),price_minor,holding.currency,stamp])?;
            let price_id = transaction.last_insert_rowid();
            let sold = closing == Some(day);
            transaction.execute("INSERT INTO daily_valuations(position_id,valuation_date,quantity_amount,quantity_scale,instrument_price_id,fx_rate_id,value_minor,currency,calculated_at) VALUES(?1,?2,?3,0,?4,?5,?6,'CHF',?7)",params![id,day.to_string(),if sold {0}else{units},price_id,if holding.currency=="USD" {Some(fx_id)}else{None},if sold {0}else{value},stamp])?;
            if day == opening || (added && day == opening + Days::days(730)) {
                let cost = if day == opening {
                    value
                } else {
                    ((holding.units / 2) as f64 * price_minor as f64 * rate).round() as i64
                };
                booking(
                    &transaction,
                    1,
                    import_cash,
                    day,
                    &format!("Wertschriftenkauf – {} (Demo)", holding.name),
                    -cost,
                    &mut cash,
                    &mut row_cash,
                    "saving",
                )?;
            }
            if sold {
                booking(
                    &transaction,
                    1,
                    import_cash,
                    day,
                    &format!("Wertschriftenverkauf – {} (Demo)", holding.name),
                    value,
                    &mut cash,
                    &mut row_cash,
                    "saving",
                )?;
            } else {
                portfolio += value;
            }
            if !sold && day.day() == 26 && day.month() % 3 == 0 {
                booking(
                    &transaction,
                    1,
                    import_cash,
                    day,
                    &format!("Ausschüttung – {} (Demo)", holding.name),
                    value / 500,
                    &mut cash,
                    &mut row_cash,
                    "income",
                )?;
            }
        }
        if day.day() == 27 {
            let interest = savings / 1500;
            booking(
                &transaction,
                2,
                import_savings,
                day,
                "Sparzins – Demo",
                interest,
                &mut savings,
                &mut row_savings,
                "income",
            )?;
        }
        for (account, import_id, balance) in [
            (1, import_cash, cash),
            (2, import_savings, savings),
            (4, import_card, card_balance),
        ] {
            transaction.execute("INSERT INTO balance_snapshots(account_id,import_id,balance_date,amount_minor,currency,recorded_at) VALUES(?1,?2,?3,?4,'CHF',?5)",params![account,import_id,day.to_string(),balance,stamp])?;
        }
        if day.month() == 12 && day.day() == 31 {
            let total = cash + savings + portfolio;
            transaction.execute("INSERT INTO annual_tax_snapshots(tax_year,valuation_date,gross_assets_minor,liabilities_minor,taxable_wealth_minor,currency,source_name,source_hash,parser_version,extraction_confidence,imported_at) VALUES(?1,?2,?3,0,?3,'CHF',?4,?5,'demo',1,?6)",params![day.year(),day.to_string(),total,format!("Demo-Steuerjahr {}",day.year()),format!("demo-tax-{}",day.year()),stamp])?;
            transaction.execute("INSERT INTO annual_tax_snapshot_breakdowns(snapshot_id,securities_and_cash_minor,real_estate_minor,other_assets_minor) VALUES(?1,?2,0,0)",params![transaction.last_insert_rowid(),total])?;
        }
        day += Days::days(1);
    }
    transaction.execute("UPDATE import_runs SET transaction_count=(SELECT COUNT(*) FROM transactions WHERE import_id=import_runs.id)",[])?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_history_reconciles_balances_holdings_and_tax_years() {
        let mut db = Connection::open_in_memory().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        seed(&mut db, today).unwrap();
        assert!(is_demo(&db));
        crate::storage::card_settlements::prepare(&db).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM matched_card_settlements", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            104
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transactions t WHERE account_id=4 AND category_id IS NULL",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(db.query_row("SELECT COUNT(DISTINCT category_id) FROM transactions WHERE account_id=4 AND amount_minor<0", [], |row| row.get::<_, i64>(0)).unwrap(), 7);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM transactions t JOIN categories c ON c.id=t.category_id WHERE t.account_id=4 AND ((description LIKE 'Coop%' AND c.category_key<>'groceries') OR (description LIKE 'Restaurant%' AND c.category_key<>'restaurants') OR (description LIKE 'SBB%' AND c.category_key<>'transport') OR (description LIKE 'Digitec%' AND c.category_key<>'electronics') OR (description LIKE 'Kino%' AND c.category_key<>'leisure') OR (description LIKE 'Netflix%' AND c.category_key<>'digital_subscriptions') OR (description LIKE 'Apotheke%' AND c.category_key<>'health'))", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM transactions t JOIN card_settlement_transactions s ON s.id=t.id WHERE t.account_id=4 AND t.amount_minor<0", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(
            db.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM accounts", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM annual_tax_snapshots", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            8
        );
        assert!(
            db.query_row("SELECT COUNT(*) FROM transactions", [], |row| row
                .get::<_, i64>(0))
                .unwrap()
                > 1000
        );
        assert_eq!(db.query_row("SELECT COUNT(*) FROM balance_snapshots s WHERE amount_minor <> CASE account_id WHEN 1 THEN 5000000 WHEN 2 THEN 1500000 ELSE 0 END + COALESCE((SELECT SUM(amount_minor) FROM transactions t WHERE t.account_id=s.account_id AND t.booking_date<=s.balance_date),0)", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM balance_snapshots WHERE account_id<>4 AND amount_minor<0",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(db.query_row("SELECT COUNT(*) FROM annual_tax_snapshots s WHERE gross_assets_minor <> (SELECT SUM(amount_minor) FROM balance_snapshots WHERE balance_date=s.valuation_date) + (SELECT SUM(value_minor) FROM daily_valuations WHERE valuation_date=s.valuation_date)", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM import_runs r WHERE transaction_count<>(SELECT COUNT(*) FROM transactions WHERE import_id=r.id)", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM daily_valuations d JOIN instrument_prices p ON p.id=d.instrument_price_id LEFT JOIN fx_rates f ON f.id=d.fx_rate_id WHERE ABS(d.value_minor-ROUND(d.quantity_amount*p.price_amount*COALESCE(f.rate_amount/1000000.0,1)))>0", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM daily_valuations WHERE valuation_date>?1",
                [today.to_string()],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(db.query_row("SELECT COUNT(*) FROM daily_valuations d JOIN portfolio_positions p ON p.id=d.position_id WHERE d.valuation_date<p.holding_start_date OR (p.holding_end_date IS NOT NULL AND d.valuation_date>p.holding_end_date)", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }

    #[test]
    fn creating_demo_preserves_existing_database_and_supports_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: directory.path().join("original.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        };
        storage.unlock("private-password".into(), true).unwrap();
        let original = fs::read(&storage.path).unwrap();
        storage
            .create_demo(NaiveDate::from_ymd_opt(2026, 9, 14).unwrap())
            .unwrap();
        assert_eq!(original, fs::read(&storage.path).unwrap());
        assert_eq!(storage.status().unwrap().database_id, "Demo 1");
        assert!(is_reusable_demo(&directory.path().join("Demo 1.vault.sqlite3"), "Demo 1"));
        assert!(!is_reusable_demo(&storage.path, "Demo 2"));
        assert!(is_demo(&storage.connect().unwrap()));
        storage.lock_session().unwrap();
        storage.unlock(PASSWORD.into(), false).unwrap();
        assert!(is_demo(&storage.connect().unwrap()));
        storage
            .connect()
            .unwrap()
            .execute_batch("CREATE TABLE demo_user_change (id INTEGER);")
            .unwrap();
        storage.select_database("original").unwrap();
        storage
            .create_demo(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap())
            .unwrap();
        assert_eq!(storage.status().unwrap().database_id, "Demo 1");
        assert!(storage
            .connect()
            .unwrap()
            .prepare("SELECT id FROM demo_user_change")
            .is_ok());
        assert!(!directory.path().join("Demo 2.vault.sqlite3").exists());
        storage.lock_session().unwrap();
        storage
            .create_demo(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap())
            .unwrap();
        assert_eq!(storage.status().unwrap().database_id, "Demo 1");
        assert_eq!(original, fs::read(&storage.path).unwrap());
    }

    #[test]
    #[ignore = "Erzeugt die importierbare Demo-Datei im Build-Verzeichnis"]
    fn export_demo_backup() {
        let today = Local::now().date_naive();
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("demo");
        fs::create_dir_all(&directory).unwrap();
        let stage = tempfile::NamedTempFile::new_in(&directory).unwrap();
        {
            let mut db = open(stage.path(), PASSWORD, true).unwrap();
            seed(&mut db, today).unwrap();
            assert_eq!(
                db.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
        }
        stage.as_file().sync_all().unwrap();
        let destination = directory.join(format!(
            "Finanzblick-Demo-Kreditkarte-{today}.finanzblick-backup"
        ));
        stage.persist_noclobber(&destination).unwrap();
        println!("Demo: {} | Passwort: {PASSWORD}", destination.display());
    }
}
