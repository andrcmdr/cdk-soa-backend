use ethers::abi::Abi;
use std::fs;

pub fn load_abi(path: &str) -> eyre::Result<Abi> {
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str::<Abi>(&data)?)
}
