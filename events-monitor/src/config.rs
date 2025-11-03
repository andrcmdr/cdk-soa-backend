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
    pub historical_logs_processing: Option<u8>,
    pub logs_sync_protocol: Option<String>,
    pub new_logs_subscription: Option<u8>,
    pub new_logs_subscription_protocol: Option<String>, // "ws" or "http", if not present in config file or 'null', then "http" by default
    pub http_polling_interval_secs: Option<u64>, // Polling interval in seconds for HTTP RPC
    pub filter_senders: Option<Vec<String>>,
    pub filter_receivers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PgCfg {
    pub dsn: String,
    pub schema: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AwsRdsCfg {
    pub enabled: Option<u8>,
    pub endpoint: String,
    pub port: Option<u16>,
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub region: Option<String>,
    pub ssl_mode: Option<String>,
    pub connection_timeout: Option<u64>,
    pub max_connections: Option<u32>,
    pub schema: Option<String>,
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
    pub name: Option<String>, // Optional name field for task identification
    pub chain: ChainCfg,
    pub indexing: IndexingCfg,
    pub postgres: PgCfg,
    pub aws_rds: Option<AwsRdsCfg>,
    pub nats: NatsCfg,
    pub contracts: Vec<ContractCfg>,
    pub max_implementations_per_contract: Option<usize>,
    pub max_implementation_nesting_depth: Option<usize>,
}

impl AppCfg {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let mut config: Self = serde_yaml::from_str(&std::fs::read_to_string(path)?)?;

        // Set default values if not specified
        if config.max_implementations_per_contract.is_none() {
            config.max_implementations_per_contract = Some(1);
        }
        if config.max_implementation_nesting_depth.is_none() {
            config.max_implementation_nesting_depth = Some(0);
        }

        // Validate implementation structure
        config.validate_implementations()?;

        Ok(config)
    }

    pub fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            format!("monitor-{}", chrono::Utc::now().timestamp())
        })
    }

    pub fn is_aws_rds_enabled(&self) -> bool {
        self.aws_rds
            .as_ref()
            .map(|rds| rds.enabled.unwrap_or(0) > 0)
            .unwrap_or(false)
    }

    fn validate_implementations(&self) -> anyhow::Result<()> {
        let max_per_contract = self.max_implementations_per_contract.unwrap_or(1);
        let max_depth = self.max_implementation_nesting_depth.unwrap_or(0);

        for contract in &self.contracts {
            self.validate_contract_implementations(contract, 0, max_per_contract, max_depth)?;
        }

        Ok(())
    }

    fn validate_contract_implementations(
        &self,
        contract: &ContractCfg,
        current_depth: usize,
        max_per_contract: usize,
        max_depth: usize,
    ) -> anyhow::Result<()> {
        if let Some(implementations) = &contract.implementations {
            if implementations.len() > max_per_contract {
                return Err(anyhow::anyhow!(
                    "Contract '{}' has {} implementations, but max allowed is {}",
                    contract.name,
                    implementations.len(),
                    max_per_contract
                ));
            }

            if current_depth >= max_depth {
                for impl_contract in implementations {
                    if impl_contract.implementations.is_some() {
                        return Err(anyhow::anyhow!(
                            "Implementation '{}' of contract '{}' has nested implementations at depth {}, but max depth is {}",
                            impl_contract.name,
                            contract.name,
                            current_depth + 1,
                            max_depth
                        ));
                    }
                }
            } else {
                for impl_contract in implementations {
                    self.validate_contract_implementations(
                        impl_contract,
                        current_depth + 1,
                        max_per_contract,
                        max_depth,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Get all contracts including implementations flattened
    pub fn get_all_contracts(&self) -> Vec<ContractWithImplementation> {
        let mut all_contracts = Vec::new();

        for contract in &self.contracts {
            self.collect_contracts_recursive(contract, None, &mut all_contracts);
        }

        all_contracts
    }

    fn collect_contracts_recursive(
        &self,
        contract: &ContractCfg,
        parent_info: Option<(String, String)>,
        collector: &mut Vec<ContractWithImplementation>,
    ) {
        // Add the current contract
        collector.push(ContractWithImplementation {
            name: contract.name.clone(),
            address: contract.address.clone(),
            abi_path: contract.abi_path.clone(),
            parent_contract_name: parent_info.as_ref().map(|(name, _)| name.clone()),
            parent_contract_address: parent_info.as_ref().map(|(_, addr)| addr.clone()),
        });

        // Add all implementations recursively
        if let Some(implementations) = &contract.implementations {
            for impl_contract in implementations {
                self.collect_contracts_recursive(
                    impl_contract,
                    Some((contract.name.clone(), contract.address.clone())),
                    collector,
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWithImplementation {
    pub name: String,
    pub address: String,
    pub abi_path: String,
    pub parent_contract_name: Option<String>,
    pub parent_contract_address: Option<String>,
}
