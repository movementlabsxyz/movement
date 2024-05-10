#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub execution_config : maptos_execution_util::config::Config,
}

impl Config {

    pub fn new(execution_config : maptos_execution_util::config::Config) -> Self {
        Self {
            execution_config,
        }
    }

    pub fn try_from_env() -> Result<Self, anyhow::Error> {

        let execution_config = maptos_execution_util::config::Config::try_from_env()?;

        Ok(Self {
            execution_config,
        })
        
    }

    pub fn write_to_env(&self) -> Result<(), anyhow::Error>{
        self.execution_config.write_to_env()?;
        Ok(())
    }

    pub fn write_bash_export_string(&self) -> Result<String, anyhow::Error> {
        Ok(format!(
            "{}",
            self.execution_config.write_bash_export_string()?
        ))
    }

}