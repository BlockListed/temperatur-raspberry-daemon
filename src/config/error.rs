#[derive(Debug)]
pub enum ConfigManagerError {
    MutexPoisoned,
    CouldntSave,
    CouldntreadConfigFile(std::io::Error),
    InvalidConfigFile(toml_edit::TomlError),
    ConfigFileMissingFields(toml_edit::de::Error),
}
