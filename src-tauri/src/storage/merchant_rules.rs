use super::*;

fn merchant_key(description: &str) -> String {
    let text = description
        .split('·')
        .next()
        .unwrap_or(description)
        .to_lowercase()
        .replace('ü', "u")
        .replace('ö', "o")
        .replace('ä', "a");
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    if words
        .iter()
        .any(|word| matches!(*word, "burgermeister" | "buergermeister"))
    {
        return "burgermeister".into();
    }
    // Keep the merchant's words; ignore varying dates and numerical references.
    words
        .into_iter()
        .filter(|word| !word.chars().any(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn apply(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS merchant_category_rules(merchant_key TEXT PRIMARY KEY, category_id INTEGER NOT NULL REFERENCES categories(id));")?;
    let rules = connection
        .prepare("SELECT merchant_key, category_id FROM merchant_category_rules")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    if rules.is_empty() {
        return Ok(());
    }
    let rows = connection
        .prepare("SELECT id, description FROM transactions")?
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, description) in rows {
        if let Some(category) = rules.get(&merchant_key(&description)) {
            connection.execute(
                "UPDATE transactions SET category_id=?1, category_source='merchant' WHERE id=?2 AND category_manual=0 AND (category_id IS NOT ?1 OR category_source<>'merchant')",
                params![category, id],
            )?;
        }
    }
    Ok(())
}

pub(super) fn learn(
    connection: &mut Connection,
    id: i64,
    category_key: &str,
) -> Result<usize, String> {
    let transaction = connection.transaction().map_err(db_error)?;
    let description: String = transaction
        .query_row(
            "SELECT description FROM transactions WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let category: i64 = transaction
        .query_row(
            "SELECT id FROM categories WHERE category_key=?1",
            [category_key],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let key = merchant_key(&description);
    transaction
        .execute(
            "UPDATE transactions SET category_id=?1, category_manual=1, category_source='manual' WHERE id=?2",
            params![category, id],
        )
        .map_err(db_error)?;
    if !key.is_empty() {
        transaction.execute("INSERT INTO merchant_category_rules(merchant_key,category_id) VALUES(?1,?2) ON CONFLICT(merchant_key) DO UPDATE SET category_id=excluded.category_id", params![key,category]).map_err(db_error)?;
    }
    apply(&transaction).map_err(db_error)?;
    let descriptions = transaction
        .prepare("SELECT description FROM transactions")
        .map_err(db_error)?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let count = if key.is_empty() {
        1
    } else {
        descriptions
            .iter()
            .filter(|text| merchant_key(text) == key)
            .count()
    };
    transaction.commit().map_err(db_error)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn learns_existing_future_and_updated_merchant_categories() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE categories(id INTEGER PRIMARY KEY,category_key TEXT); INSERT INTO categories VALUES(1,'other'),(2,'leisure'); CREATE TABLE transactions(id INTEGER PRIMARY KEY,description TEXT,category_id INTEGER,category_manual INTEGER DEFAULT 0,category_source TEXT NOT NULL DEFAULT 'description'); INSERT INTO transactions(id,description,category_id,category_manual) VALUES(1,'Bürgermeister Zürich · Einkauf: 01.01.2026',1,0),(2,'BURGERMEISTER Bern · Einkauf: 02.01.2026',1,0),(3,'Other restaurant',1,0);").unwrap();
        apply(&db).unwrap();
        assert_eq!(learn(&mut db, 1, "leisure").unwrap(), 2);
        db.execute(
            "INSERT INTO transactions(id,description,category_id,category_manual) VALUES(4,'Buergermeister Basel',1,0)",
            [],
        )
        .unwrap();
        apply(&db).unwrap();
        assert_eq!(
            db.query_row("SELECT category_id FROM transactions WHERE id=4", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(learn(&mut db, 2, "other").unwrap(), 3);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transactions WHERE category_id=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            3
        );
        assert_eq!(
            merchant_key("NETFLIX 123 · Einkauf: 01.01.2026"),
            merchant_key("Netflix 456 · Einkauf: 02.02.2026")
        );
    }
}
