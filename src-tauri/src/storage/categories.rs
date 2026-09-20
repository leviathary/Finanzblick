//! Verwaltet Kategorien, benutzerdefinierte Bezeichnungen und Zuordnungsregeln.
use crate::storage::database::errors::db_error;
#[cfg(test)]
use crate::storage::database::schema::initialize_schema;
use crate::storage::database::Storage;
use crate::storage::rules::categorization::apply_categories;
use rusqlite::{params, Connection};

use serde::Serialize;

pub(super) fn apply_redirects(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS category_redirects(source_id INTEGER PRIMARY KEY REFERENCES categories(id), target_id INTEGER NOT NULL REFERENCES categories(id));
      UPDATE transactions SET category_id=(SELECT target_id FROM category_redirects WHERE source_id=transactions.category_id)
      WHERE category_id IN (SELECT source_id FROM category_redirects);")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedCategory {
    key: String,
    label: String,
    color: String,
    transaction_count: i64,
    rule_count: i64,
}

pub fn list_categories(storage: &Storage) -> Result<Vec<ManagedCategory>, String> {
    let db = storage.connect().map_err(db_error)?;
    let mut query=db.prepare("SELECT c.category_key,c.label,c.color,
      (SELECT COUNT(*) FROM transactions t WHERE category_id=c.id
        AND NOT EXISTS (SELECT 1 FROM ignored_duplicate_transactions ignored WHERE ignored.transaction_id=t.id)),
      ((SELECT COUNT(*) FROM merchant_category_rules WHERE category_id=c.id) +
       (SELECT COUNT(*) FROM industry_category_rules WHERE category_id=c.id))
      FROM categories c WHERE c.id NOT IN (SELECT source_id FROM category_redirects) ORDER BY c.sort_order,c.label").map_err(db_error)?;
    let result = query
        .query_map([], |r| {
            Ok(ManagedCategory {
                key: r.get(0)?,
                label: r.get(1)?,
                color: r.get(2)?,
                transaction_count: r.get(3)?,
                rule_count: r.get(4)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(result)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryRule {
    industry: String,
    category_key: String,
    transaction_count: i64,
}

pub fn list_industry_rules(storage: &Storage) -> Result<Vec<IndustryRule>, String> {
    let db = storage.connect().map_err(db_error)?;
    let mut query = db
        .prepare(
            "SELECT trim(t.industry), COALESCE(rc.category_key, c.category_key), COUNT(*)
         FROM transactions t
         JOIN categories c ON c.id=t.category_id
         LEFT JOIN industry_category_rules r ON r.industry_key=lower(trim(t.industry))
         LEFT JOIN categories rc ON rc.id=r.category_id
         WHERE t.industry IS NOT NULL AND trim(t.industry)<>''
         AND NOT EXISTS (SELECT 1 FROM ignored_duplicate_transactions ignored WHERE ignored.transaction_id=t.id)
         GROUP BY lower(trim(t.industry)) ORDER BY COUNT(*) DESC, trim(t.industry)",
        )
        .map_err(db_error)?;
    let rules = query
        .query_map([], |row| {
            Ok(IndustryRule {
                industry: row.get(0)?,
                category_key: row.get(1)?,
                transaction_count: row.get(2)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rules)
}

pub fn save_industry_rule(
    storage: &Storage,
    industry: String,
    category_key: String,
) -> Result<(), String> {
    let mut db = storage.connect().map_err(db_error)?;
    let tx = db.transaction().map_err(db_error)?;
    let category_id: i64 = tx.query_row(
        "SELECT id FROM categories WHERE category_key=?1 AND id NOT IN (SELECT source_id FROM category_redirects)",
        [&category_key], |row| row.get(0)
    ).map_err(|_| "Kategorie nicht gefunden.".to_string())?;
    let label = industry.trim();
    if label.is_empty() || label.chars().count() > 120 {
        return Err("Die Branche ist ungültig.".into());
    }
    tx.execute(
        "INSERT INTO industry_category_rules(industry_key,industry_label,category_id) VALUES(lower(?1),?1,?2)
         ON CONFLICT(industry_key) DO UPDATE SET industry_label=excluded.industry_label,category_id=excluded.category_id",
        params![label, category_id]
    ).map_err(db_error)?;
    apply_categories(&tx).map_err(db_error)?;
    tx.commit().map_err(db_error)
}

fn save(
    db: &mut Connection,
    key: Option<String>,
    label: String,
    color: String,
) -> Result<(), String> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > 80 {
        return Err("Bitte einen Namen mit 1 bis 80 Zeichen eingeben.".into());
    }
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Ungültige Farbe.".into());
    }
    let tx = db.transaction().map_err(db_error)?;
    let names=tx.prepare("SELECT category_key,label FROM categories WHERE id NOT IN (SELECT source_id FROM category_redirects)").map_err(db_error)?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(db_error)?.collect::<Result<Vec<_>,_>>().map_err(db_error)?;
    if names
        .iter()
        .any(|(k, n)| Some(k) != key.as_ref() && n.to_lowercase() == label.to_lowercase())
    {
        return Err("Eine Kategorie mit diesem Namen existiert bereits.".into());
    }
    if let Some(key) = key {
        if tx.execute("UPDATE categories SET label=?1,color=?2 WHERE category_key=?3 AND id NOT IN (SELECT source_id FROM category_redirects)",params![label,color,key]).map_err(db_error)?!=1 {return Err("Kategorie nicht gefunden.".into());}
    } else {
        tx.execute("INSERT INTO categories(category_key,label,color,sort_order) VALUES('custom_' || lower(hex(randomblob(16))),?1,?2,1000)",params![label,color]).map_err(db_error)?;
    }
    tx.commit().map_err(db_error)
}

pub fn save_category(
    storage: &Storage,
    key: Option<String>,
    label: String,
    color: String,
) -> Result<(), String> {
    save(
        &mut *storage.connect().map_err(db_error)?,
        key,
        label,
        color,
    )
}

fn remove(db: &mut Connection, key: String, target_key: String) -> Result<(), String> {
    if key == target_key {
        return Err("Bitte eine andere Zielkategorie wählen.".into());
    }
    let tx = db.transaction().map_err(db_error)?;
    let lookup = |key: &str| {
        tx.query_row("SELECT id FROM categories WHERE category_key=?1 AND id NOT IN (SELECT source_id FROM category_redirects)",[key],|r|r.get::<_,i64>(0)).map_err(|_|"Kategorie nicht gefunden.".to_string())
    };
    let source = lookup(&key)?;
    let target = lookup(&target_key)?;
    tx.execute(
        "UPDATE transactions SET category_id=?1,category_manual=1,category_source='manual' WHERE category_id=?2",
        params![target, source],
    )
    .map_err(db_error)?;
    tx.execute(
        "UPDATE merchant_category_rules SET category_id=?1 WHERE category_id=?2",
        params![target, source],
    )
    .map_err(db_error)?;
    tx.execute(
        "UPDATE industry_category_rules SET category_id=?1 WHERE category_id=?2",
        params![target, source],
    )
    .map_err(db_error)?;
    tx.execute(
        "UPDATE category_redirects SET target_id=?1 WHERE target_id=?2",
        params![target, source],
    )
    .map_err(db_error)?;
    tx.execute(
        "INSERT INTO category_redirects(source_id,target_id) VALUES(?1,?2)",
        params![source, target],
    )
    .map_err(db_error)?;
    tx.commit().map_err(db_error)
}

pub fn remove_category(storage: &Storage, key: String, target_key: String) -> Result<(), String> {
    remove(&mut *storage.connect().map_err(db_error)?, key, target_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_moves_bookings_rules_and_future_classifications() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE categories(id INTEGER PRIMARY KEY,category_key TEXT); INSERT INTO categories VALUES(1,'housing'),(2,'other'); CREATE TABLE transactions(id INTEGER PRIMARY KEY,category_id INTEGER,category_manual INTEGER,category_source TEXT NOT NULL DEFAULT 'description'); INSERT INTO transactions(id,category_id,category_manual) VALUES(1,1,0); CREATE TABLE merchant_category_rules(merchant_key TEXT,category_id INTEGER); INSERT INTO merchant_category_rules VALUES('rent',1); CREATE TABLE industry_category_rules(industry_key TEXT,industry_label TEXT,category_id INTEGER);").unwrap();
        apply_redirects(&db).unwrap();
        remove(&mut db, "housing".into(), "other".into()).unwrap();
        db.execute(
            "INSERT INTO transactions(id,category_id,category_manual) VALUES(2,1,0)",
            [],
        )
        .unwrap();
        apply_redirects(&db).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transactions WHERE category_id=2",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert_eq!(
            db.query_row("SELECT category_id FROM merchant_category_rules", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn manages_categories_and_preserves_redirects_on_restart_and_import() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize_schema(&db).unwrap();
        save(&mut db, None, "Eigene Kategorie".into(), "#123456".into()).unwrap();
        assert!(save(&mut db, None, "eigene kategorie".into(), "#123456".into()).is_err());
        save(
            &mut db,
            Some("housing".into()),
            "Miete".into(),
            "#654321".into(),
        )
        .unwrap();
        db.execute("INSERT INTO merchant_category_rules SELECT 'test',id FROM categories WHERE category_key='housing'",[]).unwrap();
        remove(&mut db, "housing".into(), "other".into()).unwrap();
        remove(&mut db, "other".into(), "leisure".into()).unwrap();
        initialize_schema(&db).unwrap();
        let target:String=db.query_row("SELECT c.category_key FROM merchant_category_rules r JOIN categories c ON c.id=r.category_id WHERE merchant_key='test'",[],|r|r.get(0)).unwrap();
        assert_eq!(target, "leisure");
        assert_eq!(db.query_row("SELECT COUNT(*) FROM category_redirects r JOIN categories c ON c.id=r.target_id WHERE c.category_key='leisure'",[],|r|r.get::<_,i64>(0)).unwrap(),2);
        assert!(remove(&mut db, "leisure".into(), "leisure".into()).is_err());
    }
}
