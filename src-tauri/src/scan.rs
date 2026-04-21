use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum ReceiptFile {
    Image(PathBuf),
    Pdf(PathBuf),
}

impl ReceiptFile {
    pub fn path(&self) -> &Path {
        match self {
            ReceiptFile::Image(p) | ReceiptFile::Pdf(p) => p.as_path(),
        }
    }
}

pub fn list_receipts(dir: &Path) -> Result<Vec<ReceiptFile>> {
    let mut receipts = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("jpg") | Some("jpeg") | Some("png") | Some("webp") => {
                receipts.push(ReceiptFile::Image(path));
            }
            Some("pdf") => {
                receipts.push(ReceiptFile::Pdf(path));
            }
            _ => {}
        }
    }
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn detects_image_variants() {
        let dir = tempdir().unwrap();
        for name in &["a.jpg", "b.jpeg", "c.png", "d.webp"] {
            File::create(dir.path().join(name)).unwrap();
        }
        let receipts = list_receipts(dir.path()).unwrap();
        assert_eq!(receipts.len(), 4);
        assert!(receipts.iter().all(|r| matches!(r, ReceiptFile::Image(_))));
    }

    #[test]
    fn detects_pdf() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("receipt.pdf")).unwrap();
        let receipts = list_receipts(dir.path()).unwrap();
        assert_eq!(receipts.len(), 1);
        assert!(matches!(receipts[0], ReceiptFile::Pdf(_)));
    }

    #[test]
    fn ignores_other_extensions() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("doc.txt")).unwrap();
        File::create(dir.path().join("archive.zip")).unwrap();
        let receipts = list_receipts(dir.path()).unwrap();
        assert_eq!(receipts.len(), 0);
    }

    #[test]
    fn mixed_directory() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("a.png")).unwrap();
        File::create(dir.path().join("b.pdf")).unwrap();
        File::create(dir.path().join("c.txt")).unwrap();
        let receipts = list_receipts(dir.path()).unwrap();
        assert_eq!(receipts.len(), 2);
        let images = receipts
            .iter()
            .filter(|r| matches!(r, ReceiptFile::Image(_)))
            .count();
        let pdfs = receipts
            .iter()
            .filter(|r| matches!(r, ReceiptFile::Pdf(_)))
            .count();
        assert_eq!(images, 1);
        assert_eq!(pdfs, 1);
    }
}
