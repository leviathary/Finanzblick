//! Wendet die bestehenden Standard- und Händlerregeln auf unbestätigte Kategorien an.
use crate::storage::categories;
use crate::storage::rules::merchant_rules;
use rusqlite::Connection;

pub(crate) fn apply_categories(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "UPDATE transactions SET category_id = (
           SELECT id FROM categories WHERE category_key = CASE
             WHEN amount_minor > 0 AND account_id NOT IN (SELECT id FROM accounts WHERE account_type='credit_card') THEN 'income'
             WHEN lower(description) LIKE '%miete%' OR lower(description) LIKE '%hypothek%' THEN 'housing'
             WHEN lower(description) LIKE '%lebensmittel%' OR lower(description) LIKE '%coop%' OR lower(description) LIKE '%migros%' OR lower(description) LIKE '%haushalt%' THEN 'groceries'
             WHEN lower(description) LIKE '%krankenkasse%' OR lower(description) LIKE '%apotheke%' OR lower(description) LIKE '%arzt%' THEN 'health'
             WHEN lower(description) LIKE '%restaurant%' OR lower(description) LIKE '%restaur%'
               OR lower(description) LIKE '%fast-food%' OR lower(description) LIKE '%fast food%'
               OR lower(description) LIKE '%takeaway%' OR lower(description) LIKE '%take-away%'
               OR lower(description) LIKE '%pizzeria%' OR lower(description) LIKE '%bistro%'
               OR lower(description) LIKE '%cafÃ©%' OR lower(description) LIKE '%cafe%'
               OR lower(description) LIKE '%gastronomie%' THEN 'restaurants'
             WHEN lower(description) LIKE '%freizeit%' OR lower(description) LIKE '%kino%'
               OR lower(description) LIKE '%sport%' OR lower(description) LIKE '%fitness%'
               OR lower(description) LIKE '%gym%' OR lower(description) LIKE '%spielst%' THEN 'leisure'
             WHEN lower(description) LIKE '%sbb%' OR lower(description) LIKE '%tanken%' OR lower(description) LIKE '%mobilität%' OR lower(description) LIKE '%mobilitat%' THEN 'transport'
             WHEN lower(description) LIKE '%ferien%' OR lower(description) LIKE '%hotel%' OR lower(description) LIKE '%flug%' THEN 'travel'
             WHEN lower(description) LIKE '%steuer%' THEN 'taxes'
             WHEN lower(description) LIKE '%depot%' OR lower(description) LIKE '%vorsorge%' OR lower(description) LIKE '%säule%' OR lower(description) LIKE '%saule%' THEN 'saving'
             ELSE 'other' END
         ), category_source = 'description' WHERE category_id IS NULL;
         UPDATE transactions
           SET category_id=(SELECT id FROM categories WHERE category_key='restaurants'),
               category_source='description'
         WHERE amount_minor < 0 AND category_manual=0 AND category_source='description'
           AND category_id=(SELECT id FROM categories WHERE category_key='leisure')
           AND (lower(description) LIKE '%restaurant%' OR lower(description) LIKE '%restaur%'
             OR lower(description) LIKE '%fast-food%' OR lower(description) LIKE '%fast food%'
             OR lower(description) LIKE '%takeaway%' OR lower(description) LIKE '%take-away%'
             OR lower(description) LIKE '%pizzeria%' OR lower(description) LIKE '%bistro%'
             OR lower(description) LIKE '%cafÃ©%' OR lower(description) LIKE '%cafe%'
             OR lower(description) LIKE '%gastronomie%');
         UPDATE transactions SET category_id = (
           SELECT id FROM categories WHERE category_key = CASE
             WHEN lower(industry) LIKE '%lebensmittel%' OR lower(industry) LIKE '%supermarkt%' THEN 'groceries'
             WHEN lower(industry) LIKE '%restaurant%' OR lower(industry) LIKE '%restaur%'
               OR lower(industry) LIKE '%fast-food%' OR lower(industry) LIKE '%fast food%'
               OR lower(industry) LIKE '%gastronomie%' OR lower(industry) LIKE '%cafÃ©%'
               OR lower(industry) LIKE '%cafe%' THEN 'restaurants'
             WHEN lower(industry) LIKE '%spielst%' OR lower(industry) LIKE '%freizeit%'
               OR lower(industry) LIKE '%sport%' OR lower(industry) LIKE '%fitness%'
               OR lower(industry) LIKE '%gym%' THEN 'leisure'
             WHEN lower(industry) LIKE '%taxi%' OR lower(industry) LIKE '%transport%' OR lower(industry) LIKE '%tankstelle%' THEN 'transport'
             WHEN lower(industry) LIKE '%digitale güter%' OR lower(industry) LIKE '%digitale gueter%' THEN 'digital_subscriptions'
             WHEN lower(industry) LIKE '%hotel%' OR lower(industry) LIKE '%reise%' OR lower(industry) LIKE '%flug%' THEN 'travel'
             WHEN lower(industry) LIKE '%apotheke%' OR lower(industry) LIKE '%medizin%' OR lower(industry) LIKE '%gesundheit%' THEN 'health'
             WHEN lower(industry) LIKE '%elektronik%' THEN 'electronics'
             WHEN lower(industry) LIKE '%möbel%' OR lower(industry) LIKE '%moebel%' THEN 'furnishing'
             WHEN lower(industry) LIKE '%telekommunikation%' THEN 'telecom'
             ELSE 'other' END
         ), category_source = 'industry'
         WHERE amount_minor < 0 AND category_manual = 0 AND industry IS NOT NULL AND trim(industry) <> ''
           AND (lower(industry) LIKE '%lebensmittel%' OR lower(industry) LIKE '%supermarkt%'
             OR lower(industry) LIKE '%restaurant%' OR lower(industry) LIKE '%restaur%'
             OR lower(industry) LIKE '%fast-food%' OR lower(industry) LIKE '%fast food%'
             OR lower(industry) LIKE '%gastronomie%' OR lower(industry) LIKE '%cafÃ©%' OR lower(industry) LIKE '%cafe%'
             OR lower(industry) LIKE '%spielst%' OR lower(industry) LIKE '%freizeit%'
             OR lower(industry) LIKE '%sport%' OR lower(industry) LIKE '%fitness%' OR lower(industry) LIKE '%gym%'
             OR lower(industry) LIKE '%taxi%' OR lower(industry) LIKE '%transport%' OR lower(industry) LIKE '%tankstelle%'
             OR lower(industry) LIKE '%digitale güter%' OR lower(industry) LIKE '%digitale gueter%'
             OR lower(industry) LIKE '%hotel%' OR lower(industry) LIKE '%reise%' OR lower(industry) LIKE '%flug%'
             OR lower(industry) LIKE '%apotheke%' OR lower(industry) LIKE '%medizin%' OR lower(industry) LIKE '%gesundheit%'
             OR lower(industry) LIKE '%elektronik%' OR lower(industry) LIKE '%möbel%' OR lower(industry) LIKE '%moebel%'
             OR lower(industry) LIKE '%telekommunikation%');
         UPDATE transactions SET category_id = (
           SELECT category_id FROM industry_category_rules WHERE industry_key=lower(trim(transactions.industry))
         ), category_source = 'industry'
         WHERE amount_minor < 0 AND category_manual=0
           AND lower(trim(industry)) IN (SELECT industry_key FROM industry_category_rules);
         UPDATE transactions SET category_id = (SELECT id FROM categories WHERE category_key = 'digital_subscriptions')
           , category_source = 'description'
         WHERE amount_minor < 0 AND category_manual = 0
           AND category_id = (SELECT id FROM categories WHERE category_key = 'other')
           AND (lower(description) LIKE '%apple.com/bill%'
             OR lower(description) LIKE '%itunes.com%'
             OR lower(description) LIKE '%apple music%'
             OR lower(description) LIKE '%apple tv%'
             OR lower(description) LIKE '%icloud%'
             OR lower(description) LIKE '%google%youtube%'
             OR lower(description) LIKE '%google%one%'
             OR lower(description) LIKE '%google%storage%'
             OR lower(description) LIKE '%google%play%'
             OR lower(description) LIKE '%youtube premium%'
             OR lower(description) LIKE '%paramount+%'
             OR lower(description) LIKE '%paramountplus%'
             OR lower(description) LIKE '%netflix%'
             OR lower(description) LIKE '%disney+%'
             OR lower(description) LIKE '%disneyplus%'
             OR lower(description) LIKE '%disney plus%');
         UPDATE transactions SET category_id=(SELECT id FROM categories WHERE category_key='telecom')
           , category_source = 'description'
         WHERE amount_minor < 0 AND category_manual=0
           AND category_id=(SELECT id FROM categories WHERE category_key='other')
           AND (lower(description) LIKE '%sunrise%' OR lower(description) LIKE '%swisscom%');
         ",
    )?;
    categories::apply_redirects(connection)?;
    merchant_rules::apply(connection)
}
