use util::{
    service::Service,
    util::util::patterns::constructor::ConstructorOperations,
    util::util::version
};
use artifacts::known_artifacts::m1::testnet;

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Service;
    type Config = Config;

    fn default() -> Self::Artifact {

       Self::default_with_version(&version::Version::Latest)

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        
        Service::foreground(
            "testnet".to_string(), 
            r#"
            set -e
            echo $MOVEMENT_DIR
            $MOVEMENT_DIR/bin/avalanchego --network-id=fuji --track-subnets=$(cat $MOVEMENT_DIR/rsc/testnet-id) --plugin-dir=$MOVEMENT_DIR/avalanchego/plugins --http-host=0.0.0.0 --public-ip-resolution-service=opendns
            "#.to_string(), 
            vec![
                testnet::Constructor::default_with_version(
                    version
                ).into()
            ]
        )

    }

    fn from_config(version : &util::util::util::Version, _ : &Self::Config) -> Self::Artifact {
        Self::default_with_version(version)
    }

}
