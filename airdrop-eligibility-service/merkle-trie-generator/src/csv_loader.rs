use crate::types::AirdropEntry;
use std::fs::File;
use std::io::BufReader;
use csv::Reader;

pub fn load_airdrop_csv(path: &str) -> anyhow::Result<Vec<AirdropEntry>> {
    let file = File::open(path)?;
    let mut rdr = Reader::from_reader(BufReader::new(file));
    let mut entries = Vec::new();
    for result in rdr.deserialize() {
        let record: AirdropEntry = result?;
        entries.push(record);
    }
    Ok(entries)
}
