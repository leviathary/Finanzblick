use serde::Serialize;
use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFile {
    path: String,
    name: String,
    size: u64,
}

#[derive(Default, Serialize)]
pub struct FileSelection {
    files: Vec<ImportFile>,
    warnings: Vec<String>,
}

#[tauri::command]
pub async fn collect_import_files(
    app: tauri::AppHandle,
    paths: Vec<String>,
    recursive: bool,
) -> Result<FileSelection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let storage = app.state::<crate::storage::Storage>();
        let _lease = storage.require_unlocked()?;
        Ok(collect(paths, recursive))
    })
    .await
    .map_err(|error| error.to_string())?
}

fn collect(paths: Vec<String>, recursive: bool) -> FileSelection {
    let mut result = FileSelection::default();
    let mut pending: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let mut visited = HashSet::new();
    let mut skipped = 0;
    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                result
                    .warnings
                    .push(format!("Nicht lesbar: {}", path.display()));
                continue;
            }
        };
        // Do not traverse links into other folders or directory cycles.
        if metadata.file_type().is_symlink() {
            skipped += 1;
            continue;
        }
        let canonical = match fs::canonicalize(&path) {
            Ok(path) => path,
            Err(_) => {
                result
                    .warnings
                    .push(format!("Pfad nicht zugänglich: {}", path.display()));
                continue;
            }
        };
        if !visited.insert(canonical.clone()) {
            continue;
        }
        if metadata.is_dir() {
            match fs::read_dir(&canonical) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => match entry.file_type() {
                                Ok(kind) if !kind.is_dir() || recursive => {
                                    pending.push(entry.path())
                                }
                                Ok(_) => {}
                                Err(_) => result
                                    .warnings
                                    .push(format!("Nicht lesbar: {}", entry.path().display())),
                            },
                            Err(_) => result.warnings.push(format!(
                                "Ordner nicht vollständig lesbar: {}",
                                path.display()
                            )),
                        }
                    }
                }
                Err(_) => result
                    .warnings
                    .push(format!("Ordner nicht lesbar: {}", path.display())),
            }
            continue;
        }
        let extension = canonical
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !metadata.is_file()
            || !matches!(
                extension.as_str(),
                "xlsx" | "xls" | "csv" | "pdf" | "mt940" | "sta"
            )
        {
            skipped += 1;
            continue;
        }
        if metadata.len() > 25 * 1024 * 1024 {
            result.warnings.push(format!(
                "Grösser als 25 MB, übersprungen: {}",
                path.display()
            ));
            continue;
        }
        let Some(path_string) = canonical.to_str() else {
            result
                .warnings
                .push(format!("Dateipfad nicht darstellbar: {}", path.display()));
            continue;
        };
        result.files.push(ImportFile {
            path: path_string.to_owned(),
            name: canonical
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            size: metadata.len(),
        });
    }
    result.files.sort_by(|a, b| a.path.cmp(&b.path));
    if skipped > 0 {
        result.warnings.push(format!(
            "{skipped} nicht unterstützte Dateien oder Verknüpfungen übersprungen."
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_folders_filters_deduplicates_and_reports_failures() {
        let root =
            std::env::temp_dir().join(format!("finanzblick-file-selection-{}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("a.PDF"), b"test").unwrap();
        fs::write(root.join("ignore.txt"), b"test").unwrap();
        fs::write(root.join("nested/b.csv"), b"test").unwrap();
        let large = fs::File::create(root.join("large.xlsx")).unwrap();
        large.set_len(25 * 1024 * 1024 + 1).unwrap();
        let paths = vec![
            root.to_string_lossy().into_owned(),
            root.join("a.PDF").to_string_lossy().into_owned(),
            root.join("missing.pdf").to_string_lossy().into_owned(),
        ];
        let flat = collect(paths.clone(), false);
        assert_eq!(flat.files.len(), 1);
        assert_eq!(flat.files[0].name, "a.PDF");
        assert!(flat
            .warnings
            .iter()
            .any(|warning| warning.contains("25 MB")));
        assert!(flat
            .warnings
            .iter()
            .any(|warning| warning.contains("missing.pdf")));
        let recursive = collect(paths, true);
        assert_eq!(recursive.files.len(), 2);
        drop(large);
        fs::remove_dir_all(root).unwrap();
    }
}
