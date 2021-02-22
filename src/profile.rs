use anyhow::{Error, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::string::ToString;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct ProfileConfig {
    pub active: Option<String>,
    pub instances: Vec<InstanceConfig>,
}

impl ProfileConfig {
    pub fn get_or_create() -> Result<Self> {
        let path = get_config_path()?;
        match path.exists() {
            true => ProfileConfig::from_file(&path),
            false => ProfileConfig::init(&path),
        }
    }

    pub fn update(&self) -> Result<()> {
        let path = get_config_path()?;
        self.to_file(&path)
    }

    pub fn init(path: &PathBuf) -> Result<Self> {
        let config = Self::default();
        config.to_file(path)?;
        Ok(config)
    }

    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let file = ::std::fs::OpenOptions::new().read(true).open(path)?;
        let config: ProfileConfig = serde_yaml::from_reader(&file)?;
        Ok(config)
    }

    pub fn to_file(&self, path: &PathBuf) -> Result<()> {
        let file = ::std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;
        serde_yaml::to_writer(&file, &self)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceConfig {
    pub alias: String,
    pub instance_id: String,
    pub key_path: String,
    pub user: String,
    pub profile: String,
    pub cloud: Cloud,
}

impl ToString for InstanceConfig {
    fn to_string(&self) -> String {
        format!(
            "Alias: {}\n\
             Instance ID: {}\n\
             Key Path: {}\n\
             User: {}\n\
             Cloud: {:?}\n\
             Profile: {}",
            self.alias, self.instance_id, self.key_path, self.user, self.cloud, self.profile
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Cloud {
    Aws,
}

impl FromStr for Cloud {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aws" => Ok(Cloud::Aws),
            _ => Err(Error::msg("Unsupported cloud provider")),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let path = home_dir()
        .ok_or_else(|| Error::msg("Could not find home directory"))?
        .join(".config/remote");
    if !path.exists() {
        ::std::fs::create_dir_all(&path)?;
    };
    Ok(path.join("profiles.yaml"))
}
