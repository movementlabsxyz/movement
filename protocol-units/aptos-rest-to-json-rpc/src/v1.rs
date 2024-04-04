use rest_to_json_rpc::{
    Proxy,
    actix::{
        Actix,
        PathMatchAndExtract
    }
};

pub struct V1Proxy;

impl V1Proxy {

    pub fn try_actix() -> Result<Actix, anyhow::Error>{
        let mut actix = Actix::try_reqwest_from_env()?;

        let mut path_extractor = PathMatchAndExtract::new();

        // accounts
        path_extractor.matching(r"accounts/{address}")?;
        path_extractor.matching(r"accounts/{address}/resources")?;
        path_extractor.matching(r"accounts/{address}/modules")?;
        path_extractor.matching(r"accounts/{address}/resource{resource_type}")?;
        path_extractor.matching(r"accounts/{address}/module{module_name}")?;

        // blocks
        path_extractor.matching(r"blocks/by_height/{block_height}")?;
        path_extractor.matching(r"blocks/by_version/{version}")?;

        // events
        path_extractor.matching(r"/accounts/{address}/events/{creation_number}")?;
        path_extractor.matching(r"/accounts/{address}/events/{event_handle}/{field_name}")?;

        // general 
        path_extractor.matching(r"/spec")?;
        path_extractor.matching(r"/-/healthy")?;
        path_extractor.matching(r"/")?;

        // tables
        path_extractor.matching(r"/tables/{table_handle}/item")?;
        path_extractor.matching(r"/tables/{table_handle}/raw_item")?;

        // transactions
        path_extractor.matching(r"/transactions")?;
        path_extractor.matching(r"/transactions/by_hash/{txn_hash}")?;
        path_extractor.matching(r"/transactions/by_version/{txn_version}")?;
        path_extractor.matching(r"/accounts/{address}/transactions")?;
        path_extractor.matching(r"/transactions/batch")?;
        path_extractor.matching(r"/transactions/simulate")?;
        path_extractor.matching(r"/transactions/encode_submission")?;
        path_extractor.matching(r"/estimate_gas_price")?;

        // view
        path_extractor.matching(r"/view")?;

        actix.middleware(Box::new(path_extractor));

        Ok(actix)

    }

}