use errors;
use errors::*;
use read_input::prelude::*;
use regex::Regex;
use rusoto_core::region::Region;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use chrono::prelude::*;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub aws_mfa_device_arn: String,
    pub aws_use_default_credentials: bool,
    pub region: Option<String>,
    pub aws_temp_access_key_id: Option<String>,
    pub aws_temp_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
    pub aws_session_expiration: Option<DateTime<FixedOffset>>,
}

impl Config {
    pub fn init() -> Result<Config> {
        let mut config: Config = Config {
            region: Some(Region::EuWest1.name().to_owned()),
            aws_use_default_credentials: false,
            ..Default::default()
        };

        let use_default_creds_str: String = input::<String>()
            .msg("Use default credentials (Y/n):\n")
            .default("Y".to_owned())
            .add_test(|x| *x.to_lowercase() == "y".to_owned() || x.to_lowercase() == "n".to_owned())
            .err("That does not look like a valid response. Please try again")
            .get();

        if use_default_creds_str == "y" {
            config.aws_use_default_credentials = false;
        } else {
             config.aws_use_default_credentials = true;
        }

        if ! config.aws_use_default_credentials {
            config.aws_access_key_id = input::<String>()
                .msg("Enter your aws access key:\n")
                .get();
            config.aws_secret_access_key = input::<String>()
                .msg("Enter your aws secret key:\n")
                .get();
        }

        config.aws_mfa_device_arn = input::<String>()
            .msg("Enter mfa device arn:\n")
            .add_test(|x| x.starts_with("arn"))
            .err("That does not look like a valid response. Please try again")
            .get();


        Ok(config)
    }

    pub fn load() -> Result<(Config)> {
        let home_dir = dirs::home_dir().ok_or(ErrorKind::InvalidConfig(
            "Missing home directory".to_string(),
        ))?;
        let config_path = Path::new(home_dir.as_path()).join(".awsManager.json");

        let mut config:Config;

        if  config_path.exists() {
            let mut config_file =
                File::open(&config_path).chain_err(|| format!("could not read {:?}", config_path))?;
            let mut data = String::new();
            config_file.read_to_string(&mut data)?;
            config=serde_json::from_str(&data).chain_err(|| "Invalid json in awsManager.json")?;
        } else {
            config = Self::init()?
        }

        if config.aws_use_default_credentials {
            let (aws_access_key, aws_secret_access_key) = get_default_aws_credentials()?;
            config.aws_access_key_id = aws_access_key;
            config.aws_secret_access_key = aws_secret_access_key;
        }
        Ok(config)
    }

    pub fn persist(&self) -> Result<()> {
        let home_dir = dirs::home_dir().ok_or(ErrorKind::InvalidConfig(
            "Missing home directory".to_string(),
        ))?;
        let config_path = Path::new(home_dir.as_path()).join(".awsManager.json");
        let f = File::create(config_path)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    pub fn is_token_valid(&self) -> bool {
        if self.aws_session_token.is_none() {
            return false
        }

        return match self.aws_session_expiration {
            Some(x) => Utc::now().timestamp_millis() < x.timestamp_millis(),
            None => false,
        }

    }
}

fn get_default_aws_credentials() -> Result<(String, String)> {
    let home_dir = dirs::home_dir().ok_or(errors::ErrorKind::InvalidConfig(
        "Missing home directory".to_string(),
    ))?;
    let mut p = Path::new(home_dir.as_path()).join(".aws/credentials");
    if !p.exists() {
        let config_dir = dirs::config_dir().ok_or(errors::ErrorKind::InvalidConfig(
            "Missing config directory".to_string(),
        ))?;
        p = Path::new(config_dir.as_path()).join(".aws/credentials");
        if !p.exists() {
            Err(ErrorKind::InvalidConfig(
                "No aws credentials configuration found".to_string(),
            ))?;
        }
    }
    let config_file = File::open(p.as_path())?;
    let reader = BufReader::new(config_file);
    let mut lines = reader.lines();
    let proceed = true;
    let mut aws_access_key: Option<String> = None;
    let mut aws_secret_key: Option<String> = None;
    while proceed {
        let mut line = lines.next();
        if line.is_none() {
            break;
        }
        if line.unwrap()?.trim() == "[default]" {
            while proceed {
                line = lines.next();
                if line.is_none() {
                    break;
                }
                let line_ct = line.unwrap()?;
                if line_ct.trim().starts_with("[") || line_ct.trim() == "" {
                    break;
                }

                let re = Regex::new(r"\s*([^\s]+)\s*=\s*([^\s]+)\s*").unwrap();
                let caps = re.captures(&line_ct);
                if caps.is_none() {
                    continue;
                }
                let capture = caps.unwrap();
                let elt1 = capture.get(1).unwrap().as_str().to_string();
                let elt2 = capture.get(2).unwrap().as_str().to_string();
                if elt1 == "aws_access_key_id" {
                    aws_access_key = Some(elt2);
                } else if elt1 == "aws_secret_access_key" {
                    aws_secret_key = Some(elt2);
                }
            }
            break;
        }
    }
    if aws_secret_key.is_none() || aws_secret_key.is_none() {
        Err(ErrorKind::InvalidConfig(
            format!("No aws credentials found in {}", p.to_str().unwrap()).to_string(),
        ))?;
    }

    Ok((aws_access_key.unwrap(), aws_secret_key.unwrap()))
}
