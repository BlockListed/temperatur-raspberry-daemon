use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigManagerError {
    #[error("The Mutex was Poisoned!")]
    MutexPoisoned,
    #[error("Couldn't save data!")]
    CouldntSave,
    #[error("Could't read config file. {0}")]
    CouldntreadConfigFile(std::io::Error),
    #[error("Invalid config file! {0}")]
    InvalidConfigFile(toml_edit::TomlError),
    #[error("Config file was missing fields! {0}")]
    ConfigFileMissingFields(toml_edit::de::Error),
    #[error("Reporting interval cannot be negative")]
    ReportingIntervalNegative,
}

impl<T> From<std::sync::PoisonError<T>> for ConfigManagerError {
    fn from(_value: std::sync::PoisonError<T>) -> Self {
        Self::MutexPoisoned
    }
}