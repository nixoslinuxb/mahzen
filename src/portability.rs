//! Plain-Markdown portability: export a vault to ordinary `.md` files and
//! import a folder of Markdown (an Obsidian vault, a Git checkout, …) into an
//! encrypted vault.
//!
//! Export never touches the encrypted vault: it writes a *separate* tree of
//! decrypted notes, so the vault stays encrypted at rest while the notes become
//! readable by any other tool. Import is additive and non-destructive — files
//! are copied into the vault and encrypted there; the source folder is left
//! untouched (unlike the old in-place migration).

use std::fs;
use std::path::{Path, PathBuf};

/// Result of an export run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ExportReport {
    pub notes: usize,
    pub assets: usize,
    pub skipped: usize,
    /// Non-fatal problems (`path: error`), capped by the caller.
    pub errors: Vec<String>,
}

/// Result of an import run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub notes: usize,
    pub assets: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Whether `path` is a Markdown document we should treat as a note.
pub fn is_markdown_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

/// Collect every file under `root`, as paths relative to it, skipping hidden
/// entries, `.git`, and our own config directories.
pub fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if name == "node_modules" || name == "target" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    out.sort();
    out
}

/// Write one note's plain text to `dest`, creating parent directories.
pub fn write_plain(dest: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dest, text)
}

/// Export every note in `notes` (relative path → decrypted text) to `dest_root`,
/// copying `assets` (relative paths resolved under `vault_root`) alongside.
pub fn export_notes(
    notes: &[(PathBuf, String)],
    assets: &[(PathBuf, Vec<u8>)],
    dest_root: &Path,
) -> ExportReport {
    let mut report = ExportReport::default();
    if let Err(e) = fs::create_dir_all(dest_root) {
        report.errors.push(format!("{}: {e}", dest_root.display()));
        return report;
    }
    for (rel, text) in notes {
        // Notes are stored with a `.md` extension already; make sure.
        let mut out_rel = rel.clone();
        if !is_markdown_name(&out_rel.to_string_lossy()) {
            out_rel.set_extension("md");
        }
        let dest = dest_root.join(&out_rel);
        match write_plain(&dest, text) {
            Ok(()) => report.notes += 1,
            Err(e) => report.errors.push(format!("{}: {e}", out_rel.display())),
        }
    }
    for (rel, bytes) in assets {
        let dest = dest_root.join(rel);
        if let Some(parent) = dest.parent() {
            if fs::create_dir_all(parent).is_err() {
                report.skipped += 1;
                continue;
            }
        }
        match fs::write(&dest, bytes) {
            Ok(()) => report.assets += 1,
            Err(e) => {
                report.skipped += 1;
                report.errors.push(format!("{}: {e}", rel.display()));
            }
        }
    }
    report
}

/// A Markdown file discovered for import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFile {
    /// Path relative to the import root (kept so folders map 1:1).
    pub rel: PathBuf,
    /// Absolute source path.
    pub source: PathBuf,
}

/// Scan an import root and classify its files. `.md`/`.markdown` files become
/// notes; everything else is treated as an attachment.
pub fn scan_import(root: &Path) -> (Vec<ImportFile>, Vec<PathBuf>) {
    let mut notes = Vec::new();
    let mut assets = Vec::new();
    for rel in walk_files(root) {
        let source = root.join(&rel);
        let name = rel.file_name().map(|n| n.to_string_lossy().to_string());
        let Some(name) = name else { continue };
        if is_markdown_name(&name) {
            notes.push(ImportFile { rel, source });
        } else {
            assets.push(rel);
        }
    }
    (notes, assets)
}

/// Pick a non-clashing destination for `rel` inside `vault_root`, appending
/// ` (n)` before the extension when needed.
pub fn unique_target(vault_root: &Path, rel: &Path) -> PathBuf {
    let mut candidate = vault_root.join(rel);
    if !candidate.exists() {
        return candidate;
    }
    let stem = rel
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "note".into());
    let ext = rel
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "md".into());
    let parent = rel.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut n = 1;
    loop {
        let name = format!("{stem} ({n}).{ext}");
        candidate = vault_root.join(&parent).join(name);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Import `files` into `vault_root`. `read_text` returns a note's plaintext
/// (decrypting if needed) and `write_note` stores it encrypted. Assets are
/// copied verbatim. The source tree is never modified.
#[allow(dead_code)]
pub fn import_files(
    files: &[ImportFile],
    assets: &[PathBuf],
    import_root: &Path,
    vault_root: &Path,
    read_text: &mut dyn FnMut(&ImportFile) -> Option<String>,
    write_note: &mut dyn FnMut(&Path, &str) -> std::io::Result<()>,
) -> ImportReport {
    let mut report = ImportReport::default();
    for file in files {
        let Some(text) = read_text(file) else {
            report.skipped += 1;
            report
                .errors
                .push(format!("{}: could not read", file.rel.display()));
            continue;
        };
        let dest = unique_target(vault_root, &file.rel);
        match write_note(&dest, &text) {
            Ok(()) => report.notes += 1,
            Err(e) => {
                report.skipped += 1;
                report.errors.push(format!("{}: {e}", file.rel.display()));
            }
        }
    }
    for rel in assets {
        let src = import_root.join(rel);
        let dest = unique_target(vault_root, rel);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::copy(&src, &dest) {
            Ok(_) => report.assets += 1,
            Err(e) => {
                report.skipped += 1;
                report.errors.push(format!("{}: {e}", rel.display()));
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "encd-port-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn exports_notes_and_assets_as_plain_markdown() {
        let vault = tmp("export-vault");
        let dest = tmp("export-dest");
        fs::create_dir_all(vault.join("assets")).unwrap();
        fs::write(vault.join("assets/a.png"), b"png").unwrap();

        let notes = vec![
            (PathBuf::from("Note.md"), "# Hello\n\nbody\n".to_string()),
            (
                PathBuf::from("Sub/Other.md"),
                "---\ntags: [x]\n---\ntext\n".to_string(),
            ),
        ];
        let report = export_notes(
            &notes,
            &[(PathBuf::from("assets/a.png"), b"png".to_vec())],
            &dest,
        );
        assert_eq!(report.notes, 2);
        assert_eq!(report.assets, 1);
        assert_eq!(
            fs::read_to_string(dest.join("Note.md")).unwrap(),
            "# Hello\n\nbody\n"
        );
        assert!(dest.join("Sub/Other.md").exists());
        assert_eq!(fs::read(dest.join("assets/a.png")).unwrap(), b"png");
        let _ = fs::remove_dir_all(&vault);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn scans_and_classifies_import_files() {
        let root = tmp("scan");
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("A.md"), b"# a").unwrap();
        fs::write(root.join("notes/B.markdown"), b"# b").unwrap();
        fs::write(root.join("notes/pic.png"), b"x").unwrap();
        fs::write(root.join(".git/config"), b"x").unwrap();
        fs::write(root.join(".hidden.md"), b"x").unwrap();
        let (notes, assets) = scan_import(&root);
        assert_eq!(notes.len(), 2);
        assert_eq!(assets, vec![PathBuf::from("notes/pic.png")]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_is_non_destructive_and_renames_clashes() {
        let src = tmp("import-src");
        let vault = tmp("import-vault");
        fs::write(src.join("A.md"), b"# new a").unwrap();
        fs::write(vault.join("A.md"), b"existing").unwrap();
        let (notes, assets) = scan_import(&src);
        let mut store = std::collections::HashMap::new();
        let report = import_files(
            &notes,
            &assets,
            &src,
            &vault,
            &mut |f| fs::read_to_string(&f.source).ok(),
            &mut |p, t| {
                store.insert(p.to_path_buf(), t.to_string());
                fs::write(p, t)
            },
        );
        assert_eq!(report.notes, 1);
        // Original source file is untouched.
        assert_eq!(fs::read(src.join("A.md")).unwrap(), b"# new a");
        // Existing vault note is untouched; the import got a new name.
        assert_eq!(fs::read(vault.join("A.md")).unwrap(), b"existing");
        assert!(vault.join("A (1).md").exists());
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&vault);
    }
}
