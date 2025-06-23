use ethers::{providers::StreamExt, types::Log};
use tokio;

const RPC: &str = "http://localhost:8545";
const STREAM: &str = "ws://localhost:8000/stream";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let ws = ethers::providers::Ws::connect(STREAM).await?;
    let mut sub = ws.subscribe_blocks().await?;

    while let Some(block) = sub.next().await {
        for tx in block.transactions {
            for log in tx.logs {
                if log.topics.get(0)
                       == Some(&"0x".to_owned() + &ethers::utils::keccak256("ValueSet(address,uint256)").to_hex::<String>())
                {
                    let setter = ethers::types::H160::from_slice(&log.topics[1][12..]);
                    let value = ethers::types::U256::from_big_endian(&log.data.0);
                    println!("ValueSet by {setter:?} = {value}");
                    // insert into DB
                }
            }
        }
    }
    Ok(())
}
