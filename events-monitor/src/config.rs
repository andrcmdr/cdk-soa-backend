use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ChainCfg {
    pub http_rpc_url: String,
    pub ws_rpc_url: String,
    pub chain_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexingCfg {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PgCfg {
    pub dsn: String,
    pub schema: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NatsCfg {
    pub nats_enabled: Option<u8>,
    pub url: String,
    pub object_store_bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractCfg {
    pub name: String,
    pub address: String,
    pub abi_path: String,
    pub implementations: Option<Vec<ContractCfg>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppCfg {
    pub chain: ChainCfg,
    pub indexing: IndexingCfg,
    pub postgres: PgCfg,
    pub nats: NatsCfg,
    pub contracts: Vec<ContractCfg>,
    pub max_implementations_per_contract: Option<usize>,
    pub max_implementation_nesting_depth: Option<usize>,
}

impl AppCfg {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let mut config: AppCfg = serde_yaml::from_str(&std::fs::read_to_string(path)?)?;

        // Set default values
        if config.max_implementations_per_contract.is_none() {
            config.max_implementations_per_contract = Some(1);
        }
        if config.max_implementation_nesting_depth.is_none() {
            config.max_implementation_nesting_depth = Some(0);
        }

        // Validate configuration
        Self::validate_contracts(&config.contracts, 0, &config)?;

        Ok(config)
    }

    fn validate_contracts(contracts: &[ContractCfg], current_depth: usize, config: &AppCfg) -> anyhow::Result<()> {
        let max_depth = config.max_implementation_nesting_depth.unwrap_or(0);
        let max_impls = config.max_implementations_per_contract.unwrap_or(1);

        for contract in contracts {
            if let Some(implementations) = &contract.implementations {
                // Check max implementations limit
                if implementations.len() > max_impls {
                    return Err(anyhow::anyhow!(
                        "Contract '{}' has {} implementations, but maximum allowed is {}",
                        contract.name, implementations.len(), max_impls
                    ));
                }

                // Check nesting depth
                if current_depth >= max_depth {
                    return Err(anyhow::anyhow!(
                        "Implementation nesting depth {} exceeds maximum allowed depth of {}",
                        current_depth + 1, max_depth
                    ));
                }

                // Validate that implementations don't have their own implementations at max depth
                for implementation in implementations {
                    if implementation.implementations.is_some() && current_depth >= max_depth {
                        return Err(anyhow::anyhow!(
                            "Implementation '{}' has nested implementations at maximum depth {}",
                            implementation.name, max_depth
                        ));
                    }
                }

                // Recursively validate nested implementations
                Self::validate_contracts(implementations, current_depth + 1, config)?;
            }
        }

        Ok(())
    }

    /// Flatten all contracts and their implementations into a single list
    pub fn get_all_contracts(&self) -> Vec<FlattenedContract> {
        let mut flattened = Vec::new();
        self.flatten_contracts(&self.contracts, None, &mut flattened);
        flattened
    }

    fn flatten_contracts(&self, contracts: &[ContractCfg], parent_contract: Option<&ContractCfg>, flattened: &mut Vec<FlattenedContract>) {
        for contract in contracts {
            flattened.push(FlattenedContract {
                name: contract.name.clone(),
                address: contract.address.clone(),
                abi_path: contract.abi_path.clone(),
                parent_contract_name: parent_contract.map(|p| p.name.clone()),
                parent_contract_address: parent_contract.map(|p| p.address.clone()),
                is_implementation: parent_contract.is_some(),
            });

            if let Some(implementations) = &contract.implementations {
                self.flatten_contracts(implementations, Some(contract), flattened);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlattenedContract {
    pub name: String,
    pub address: String,
    pub abi_path: String,
    pub parent_contract_name: Option<String>,
    pub parent_contract_address: Option<String>,
    pub is_implementation: bool,
}
