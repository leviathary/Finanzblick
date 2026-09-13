use rusqlite::Connection;

// Match across the full history, before applying display filters. Only unique
// pairs qualify; equal amounts alone never establish an internal transfer.
pub(super) fn prepare(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TEMP TABLE matched_card_settlements AS
        WITH candidates AS (
          SELECT d.id AS debit_id, c.id AS credit_id
          FROM transactions d JOIN accounts da ON da.id=d.account_id
          JOIN transactions c ON c.amount_minor=-d.amount_minor AND c.currency=d.currency
          JOIN accounts ca ON ca.id=c.account_id
          WHERE d.amount_minor<0 AND da.account_type<>'credit_card'
            AND ca.account_type='credit_card' AND da.is_active=1 AND ca.is_active=1
            AND ABS(julianday(c.booking_date)-julianday(d.booking_date))<=7
            AND (lower(d.description) LIKE '%kreditkart%' OR lower(d.description) LIKE '%credit card%'
                 OR lower(d.description) LIKE '%vis1w%' OR lower(d.description) LIKE '%card center%')
            AND (lower(c.description) LIKE '%lsv%zahlung%' OR lower(c.description) LIKE '%zahlung erhalten%'
                 OR lower(c.description) LIKE '%payment received%' OR lower(c.description) LIKE '%rechnungsausgleich%')
        )
        SELECT debit_id, credit_id FROM candidates x
        WHERE (SELECT COUNT(*) FROM candidates y WHERE y.debit_id=x.debit_id)=1
          AND (SELECT COUNT(*) FROM candidates y WHERE y.credit_id=x.credit_id)=1;")?;

    // Label statement payments even when the other side was not imported.
    // Keep this separate from pairing: an identified payment does not prove a match.
    // Do not use editable categories, amount alone, or generic bank/LSV descriptions.
    db.execute_batch("CREATE TEMP TABLE card_settlement_transactions AS
        SELECT debit_id AS id FROM matched_card_settlements
        UNION SELECT credit_id FROM matched_card_settlements
        UNION SELECT t.id FROM transactions t JOIN accounts a ON a.id=t.account_id
        WHERE a.is_active=1 AND (
          (a.account_type<>'credit_card' AND t.amount_minor<0 AND (
            (lower(t.description) LIKE '%ubs%vis1w%' AND
              (lower(t.description) LIKE '%widerspruch%' OR lower(t.description) LIKE '%lastschrift%'))
            OR lower(t.description) LIKE '%kreditkartenabrechnung%'
            OR lower(t.description) LIKE '%kreditkarten-abrechnung%'
            OR lower(t.description) LIKE '%credit card payment%'
          ))
          OR (a.account_type='credit_card' AND t.amount_minor>0 AND (
            lower(t.description) LIKE '%lsv%zahlung%'
            OR lower(t.description) LIKE '%zahlung erhalten%'
            OR lower(t.description) LIKE '%payment received%'
            OR lower(t.description) LIKE '%rechnungsausgleich%'
          ))
        );")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_only_unique_card_payment_pairs() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE accounts(id INTEGER,account_type TEXT,is_active INTEGER);
            INSERT INTO accounts VALUES(1,'checking',1),(2,'credit_card',1);
            CREATE TABLE transactions(id INTEGER,account_id INTEGER,amount_minor INTEGER,currency TEXT,booking_date TEXT,description TEXT);
            INSERT INTO transactions VALUES
              (1,1,-417250,'CHF','2026-05-27','UBS VIS1W Lastschrift'),
              (2,2,417250,'CHF','2026-05-29','2002 LSV-ZAHLUNG'),
              (3,2,-417250,'CHF','2026-05-01','Einkauf'),
              (4,1,-10000,'CHF','2026-05-27','Andere Rechnung'),
              (5,2,10000,'CHF','2026-05-28','LSV-ZAHLUNG'),
              (6,1,-20000,'CHF','2026-05-27','Kreditkarte'),
              (7,2,20000,'CHF','2026-05-28','LSV-ZAHLUNG'),
              (8,2,20000,'CHF','2026-05-29','LSV-ZAHLUNG'),
              (9,1,-30000,'CHF','2026-05-01','Kreditkarte'),
              (10,2,30000,'CHF','2026-05-20','LSV-ZAHLUNG'),
              (11,1,-40000,'CHF','2026-05-27','Kreditkarte'),
              (12,2,40000,'EUR','2026-05-28','LSV-ZAHLUNG');").unwrap();
        prepare(&db).unwrap();
        let ids = db
            .prepare("SELECT debit_id FROM matched_card_settlements")
            .unwrap()
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(ids, vec![1]);
    }
}
