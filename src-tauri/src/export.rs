use crate::receipt::ReceiptRecord;
use anyhow::Result;
use std::path::Path;

const DEFAULT_COLUMNS: &[&str] = &[
    "date",
    "category",
    "entity",
    "amount",
    "payment_method",
];

pub fn default_columns() -> Vec<String> {
    DEFAULT_COLUMNS.iter().map(|s| s.to_string()).collect()
}

pub fn export_csv(
    records: Vec<ReceiptRecord>,
    columns: Vec<String>,
    output_path: &Path,
) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_path)?;
    wtr.write_record(&columns)?;
    for record in &records {
        let row: Vec<String> = columns.iter().map(|col| field_value(record, col)).collect();
        wtr.write_record(&row)?;
    }
    wtr.flush()?;
    Ok(())
}

fn field_value(record: &ReceiptRecord, column: &str) -> String {
    match column {
        "date" => record.date.clone(),
        "category" => record.category.clone(),
        "entity" => record.entity.clone(),
        "amount" => record.amount.clone(),
        "payment_method" => record.payment_method.clone(),
        "source_path" => record.source_path.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_records() -> Vec<ReceiptRecord> {
        vec![
            ReceiptRecord {
                date: "2024-01-15".into(),
                category: "Food".into(),
                entity: "Starbucks".into(),
                amount: "$5.50".into(),
                payment_method: "Visa".into(),
                source_path: "/tmp/receipt1.jpg".into(),
            },
            ReceiptRecord {
                date: "2024-01-16".into(),
                category: "Travel".into(),
                entity: "Uber".into(),
                amount: "$12.00".into(),
                payment_method: "MasterCard".into(),
                source_path: "/tmp/receipt2.pdf".into(),
            },
        ]
    }

    #[test]
    fn exports_default_columns() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        let records = sample_records();
        let cols = default_columns();
        export_csv(records, cols, &out).unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let headers: Vec<String> = rdr
            .headers()
            .unwrap()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            headers,
            vec!["date", "category", "entity", "amount", "payment_method"]
        );
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get(0).unwrap(), "2024-01-15");
        assert_eq!(rows[1].get(2).unwrap(), "Uber");
    }

    #[test]
    fn exports_custom_column_order() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        let records = sample_records();
        let cols = vec!["amount".into(), "entity".into(), "date".into()];
        export_csv(records, cols, &out).unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let headers: Vec<String> = rdr
            .headers()
            .unwrap()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(headers, vec!["amount", "entity", "date"]);
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows[0].get(0).unwrap(), "$5.50");
        assert_eq!(rows[0].get(1).unwrap(), "Starbucks");
    }

    #[test]
    fn unknown_column_yields_empty_string() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        let records = sample_records();
        let cols = vec!["date".into(), "notes".into()];
        export_csv(records, cols, &out).unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows[0].get(1).unwrap(), "");
    }

    #[test]
    fn empty_records_writes_only_header() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.csv");
        export_csv(vec![], default_columns(), &out).unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
        assert_eq!(rows.len(), 0);
    }
}
