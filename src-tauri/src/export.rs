use crate::receipt::ReceiptRecord;
use anyhow::Result;
use std::io::Write as _;
use std::path::Path;

#[must_use]
pub fn default_keys() -> Vec<String> {
    Vec::new()
}

pub fn export_csv(
    records: Vec<ReceiptRecord>,
    keys: Vec<String>,
    output_path: &Path,
) -> Result<()> {
    let mut file = std::fs::File::create(output_path)?;
    // UTF-8 BOM so Excel recognises the encoding instead of falling back to
    // the system locale, which garbles Japanese/accented characters.
    file.write_all(b"\xEF\xBB\xBF")?;
    let mut wtr = csv::Writer::from_writer(file);
    wtr.write_record(&keys)?;
    for record in &records {
        let row: Vec<String> = keys
            .iter()
            .map(|key| record.fields.get(key).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&row)?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn make_record(path: &str, pairs: &[(&str, &str)]) -> ReceiptRecord {
        let mut fields = HashMap::new();
        for (k, v) in pairs {
            fields.insert(k.to_string(), v.to_string());
        }
        ReceiptRecord { source_path: path.to_string(), fields }
    }

    fn read_csv_bytes(path: &std::path::Path) -> Vec<u8> {
        let bytes = std::fs::read(path).unwrap();
        // Strip the UTF-8 BOM that export_csv writes for Excel compatibility.
        if bytes.starts_with(b"\xEF\xBB\xBF") { bytes[3..].to_vec() } else { bytes }
    }

    #[test]
    fn exports_schema_keys() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        let records = vec![
            make_record("/tmp/r1.jpg", &[("date", "2024-01-15"), ("amount", "$5.50")]),
            make_record("/tmp/r2.jpg", &[("date", "2024-01-16"), ("amount", "$12.00")]),
        ];
        let keys = vec!["date".to_string(), "amount".to_string()];
        export_csv(records, keys, &out).unwrap();

        let content = read_csv_bytes(&out);
        let mut rdr = csv::Reader::from_reader(content.as_slice());
        let headers: Vec<String> = rdr
            .headers()
            .unwrap()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(headers, vec!["date", "amount"]);
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get(0).unwrap(), "2024-01-15");
        assert_eq!(rows[1].get(1).unwrap(), "$12.00");
    }

    #[test]
    fn unknown_key_yields_empty_string() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        let records = vec![make_record("/tmp/r1.jpg", &[("date", "2024-01-15")])];
        let keys = vec!["date".to_string(), "notes".to_string()];
        export_csv(records, keys, &out).unwrap();

        let content = read_csv_bytes(&out);
        let mut rdr = csv::Reader::from_reader(content.as_slice());
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows[0].get(1).unwrap(), "");
    }

    #[test]
    fn output_starts_with_utf8_bom() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        export_csv(vec![], vec!["date".to_string()], &out).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"\xEF\xBB\xBF"), "missing UTF-8 BOM for Excel");
    }

    #[test]
    fn empty_records_writes_only_header() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        export_csv(vec![], vec!["date".to_string()], &out).unwrap();

        let content = read_csv_bytes(&out);
        let mut rdr = csv::Reader::from_reader(content.as_slice());
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows.len(), 0);
    }
}
