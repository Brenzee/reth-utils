use std::{path::Path, sync::Arc};

use reth::{
    api::NodeTypesWithDBAdapter,
    providers::{providers::StaticFileProvider, ProviderFactory, StateProvider},
};
use reth_chainspec::ChainSpecBuilder;
use reth_db::{mdbx::DatabaseArguments, open_db_read_only, ClientVersion, DatabaseEnv};
use reth_node_ethereum::EthereumNode;

pub struct Config {
    factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
}

impl Config {
    pub fn new(db_path: &Path) -> eyre::Result<Self> {
        let db = Arc::new(open_db_read_only(
            db_path.join("db").as_path(),
            DatabaseArguments::new(ClientVersion::default()),
        )?);

        let spec = Arc::new(ChainSpecBuilder::mainnet().build());

        let static_files = StaticFileProvider::read_only(db_path.join("static_files"), true)?;

        let factory =
            ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
                db.clone(),
                spec.clone(),
                static_files,
            );

        Ok(Self { factory })
    }

    pub fn factory(
        &self,
    ) -> &ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>> {
        &self.factory
    }

    pub fn get_latest_state(&self) -> Box<dyn StateProvider> {
        self.factory.latest().unwrap()
    }
}
