//! Tauri-Eingänge für Kategorien und Branchenregeln.
use crate::storage::categories::{self as repository, CategoryRule, IndustryRule, ManagedCategory};
use crate::storage::Storage;
use tauri::State;
#[tauri::command]
pub fn list_categories(storage: State<'_, Storage>) -> Result<Vec<ManagedCategory>, String> {
    repository::list_categories(&storage)
}
#[tauri::command]
pub fn list_category_rules(
    storage: State<'_, Storage>,
    category_key: String,
) -> Result<Vec<CategoryRule>, String> {
    repository::list_category_rules(&storage, category_key)
}
#[tauri::command]
pub fn delete_category_rule(
    storage: State<'_, Storage>,
    category_key: String,
    rule_type: String,
    label: String,
) -> Result<usize, String> {
    repository::delete_category_rule(&storage, category_key, rule_type, label)
}
#[tauri::command]
pub fn list_industry_rules(storage: State<'_, Storage>) -> Result<Vec<IndustryRule>, String> {
    repository::list_industry_rules(&storage)
}
#[tauri::command]
pub fn save_industry_rule(
    storage: State<'_, Storage>,
    industry: String,
    category_key: String,
) -> Result<(), String> {
    repository::save_industry_rule(&storage, industry, category_key)
}
#[tauri::command]
pub fn save_category(
    storage: State<'_, Storage>,
    key: Option<String>,
    label: String,
    color: String,
) -> Result<(), String> {
    repository::save_category(&storage, key, label, color)
}
#[tauri::command]
pub fn remove_category(
    storage: State<'_, Storage>,
    key: String,
    target_key: String,
) -> Result<(), String> {
    repository::remove_category(&storage, key, target_key)
}
