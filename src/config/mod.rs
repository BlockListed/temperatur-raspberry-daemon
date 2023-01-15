use std::fs::read;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use toml_edit::{de, value, Document};

pub mod error;
use error::ConfigManagerError;

#[derive(Deserialize)]
pub struct Configuration {
    pub endpoint: String,
    pub data_script: String,
    pub node_endpoint: String,
    pub graphana_endpoint: String,
    pub reporting_interval: Mutex<f64>,
    pub raum_id: i32,
}

pub struct ConfigManager {
    path: String,
    doc: Mutex<Document>,
    conf: Arc<Configuration>,
}

impl ConfigManager {
    #[tracing::instrument]
    pub fn new(path: String) -> Result<Self, ConfigManagerError> {
        // Parse document from `path`
        let doc = match Document::from_str(
            std::str::from_utf8(
                match read(&path) {
                    Ok(x) => x,
                    Err(x) => {
                        tracing::error!("Couldn't read config file!");
                        return Err(ConfigManagerError::CouldntreadConfigFile(x));
                    }
                }
                .as_slice(),
            )
            .unwrap(),
        ) {
            Ok(x) => x,
            Err(x) => {
                tracing::error!("Invalid config file!!");
                return Err(ConfigManagerError::InvalidConfigFile(x));
            }
        };
        let conf: Configuration = match de::from_document(doc.clone()) {
            Ok(x) => x,
            Err(x) => {
                tracing::error!("Config file missing fields!");
                return Err(ConfigManagerError::ConfigFileMissingFields(x));
            }
        };

        Ok(Self {
            path,
            doc: Mutex::new(doc),
            conf: Arc::new(conf),
        })
    }

    pub fn configuration(&self) -> Arc<Configuration> {
        Arc::clone(&self.conf)
    }

    pub fn update_reporting_interval(&self, seconds: f64) -> Result<(), ConfigManagerError> {
        if seconds < 0.0 {
            return Err(ConfigManagerError::ReportingIntervalNegative);
        }
        let mut doc = match self.doc.lock() {
            Ok(x) => x,
            Err(_) => return Err(ConfigManagerError::MutexPoisoned),
        };
        doc["reporting_interval"] = value(seconds);
        // Dropping since it triggers a deadlock otherwise
        drop(doc);

        let mut interval = match self.conf.reporting_interval.lock() {
            Ok(x) => x,
            Err(_) => return Err(ConfigManagerError::MutexPoisoned),
        };
        *interval = seconds;
        drop(interval);

        self.save()?;
        Ok(())
    }

    fn save(&self) -> Result<(), ConfigManagerError> {
        let doc = match self.doc.lock() {
            Ok(x) => x,
            Err(_) => return Err(ConfigManagerError::MutexPoisoned),
        };
        if std::fs::write(&self.path, doc.clone().to_string()).is_err() {
            return Err(ConfigManagerError::CouldntSave);
        }
        Ok(())
    }
}
