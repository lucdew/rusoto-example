use anyhow::Context;
use anyhow::Result;
use chrono::prelude::*;
use dialoguer::{Confirm, Input};
use regex::Regex;
use rusoto_core::region::Region;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub aws_mfa_device_arn: Option<String>,
    pub aws_use_default_credentials: bool,
    pub region: Option<String>,
    pub aws_sts_profile: Option<String>,
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

        if Confirm::new()
            .with_prompt("Use default credentials?")
            .interact()?
        {
            config.aws_use_default_credentials = true;
        }

        if !config.aws_use_default_credentials {
            config.aws_access_key_id = Input::<String>::new()
                .with_prompt("Enter your aws access key")
                .interact()?;
            config.aws_secret_access_key = Input::<String>::new()
                .with_prompt("Enter your aws secret key")
                .interact()?;
        }

        Ok(config)
    }

    pub fn load() -> Result<Config> {
        let home_dir = dirs::home_dir().context("Missing home directory")?;
        let config_path = Path::new(home_dir.as_path()).join(".awsManager.json");

        let mut config: Config;

        if config_path.exists() {
            let mut config_file = File::open(&config_path)
                .with_context(|| format!("could not read {:?}", config_path))?;
            let mut data = String::new();
            config_file.read_to_string(&mut data)?;
            config = serde_json::from_str(&data).context("Invalid json in awsManager.json")?;
        } else {
            config = Self::init()?
        }

        if config.aws_use_default_credentials {
            set_default_aws_credentials(&mut config)?;
        }

        if config.aws_sts_profile.is_none() && config.aws_mfa_device_arn.is_none() {
            config.aws_mfa_device_arn = Some(
                Input::<String>::new()
                    .with_prompt("Enter your MFA device ARN")
                    .validate_with(|input: &str| -> Result<(), &str> {
                        if input.starts_with("arn") {
                            Ok(())
                        } else {
                            Err("This is not a valid mfa arn")
                        }
                    })
                    .interact()?,
            );
        }
        Ok(config)
    }

    pub fn persist(&self) -> Result<()> {
        let home_dir = dirs::home_dir().context("Missing home directory")?;
        let config_path = Path::new(home_dir.as_path()).join(".awsManager.json");
        let f = File::create(config_path)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    pub fn is_token_valid(&self) -> bool {
        if self.aws_session_token.is_none() {
            return false;
        }

        if self.aws_sts_profile.is_some() {
            // rely completely on the token from .aws/credentials
            return true;
        }

        return match self.aws_session_expiration {
            Some(x) => Utc::now().timestamp_millis() < x.timestamp_millis(),
            None => false,
        };
    }
}

fn set_default_aws_credentials(cfg: &mut Config) -> Result<()> {
    let home_dir = dirs::home_dir().context("Missing home directory")?;
    let mut p = Path::new(home_dir.as_path()).join(".aws/credentials");
    if !p.exists() {
        let config_dir = dirs::config_dir().context("Missing config directory")?;
        p = Path::new(config_dir.as_path()).join(".aws/credentials");
        if !p.exists() {
            Err(anyhow!("No aws credentials configuration found"))?;
        }
    }
    let config_file = File::open(p.as_path())?;
    let reader = BufReader::new(config_file);
    let mut lines = reader.lines();
    let mut aws_access_key: Option<String> = None;
    let mut aws_secret_key: Option<String> = None;

    let mut profiles_cfg_map: HashMap<String, HashMap<String, Option<String>>> = HashMap::new();

    let mut profile_cfg: HashMap<String, Option<String>> = HashMap::new();

    let kv_regex = Regex::new(r"\s*([^\s]+)\s*=\s*([^\s]*)\s*").unwrap();
    let profile_regex = Regex::new(r"\[([^\]]+)\]").unwrap();
    let mut current_profile: Option<String> = None;
    loop {
        let line_opt = lines.next();

        if let Some(line_res) = line_opt {
            let raw_line = line_res?;
            let line = raw_line.trim();
            debug!("Parsing line {}", &line);
            let mut caps = profile_regex.captures(&line);
            if let Some(profile_cap) = caps {
                if let Some(profile) = current_profile {
                    debug!("Added profile {} cfg {:?}", &profile, &profile_cfg);
                    profiles_cfg_map.insert(profile, profile_cfg);
                    profile_cfg = HashMap::new();
                }
                current_profile = Some(profile_cap.get(1).unwrap().as_str().to_owned());
                continue;
            }
            caps = kv_regex.captures(&line);
            if let Some(kv_cap) = caps {
                let k = kv_cap.get(1).unwrap().as_str().to_owned();
                let v = match kv_cap.get(2) {
                    Some(v) => Some(v.as_str().to_owned()),
                    None => None,
                };
                profile_cfg.insert(k, v);
            }
        } else {
            if let Some(profile) = current_profile {
                debug!("Added profile {} cfg {:?}", &profile, &profile_cfg);
                profiles_cfg_map.insert(profile, profile_cfg);
            }
            debug!("No more line");
            break;
        }
    }
    if let Some(default_profile_cfg) = profiles_cfg_map.get("default") {
        if let Some(aws_access_key_opt) = default_profile_cfg.get("aws_access_key_id") {
            aws_access_key = aws_access_key_opt.clone();
        }
        if let Some(aws_secret_key_opt) = default_profile_cfg.get("aws_secret_access_key") {
            aws_secret_key = aws_secret_key_opt.clone();
        }
    }
    for (profile_name, profile_cfg) in &profiles_cfg_map {
        debug!("Parsing profile {}", profile_name);
        if let Some(aws_session_token_opt) = profile_cfg.get("aws_session_token") {
            if aws_session_token_opt.is_some() {
                debug!("Found aws_session_token for profile {}", &profile_name);
                cfg.aws_sts_profile = Some(profile_name.clone());
                cfg.aws_session_token = aws_session_token_opt.clone();
            }
            if let Some(aws_access_key) =
                profile_cfg.get("aws_access_key_id").and_then(|v| v.clone())
            {
                debug!("Found aws_temp_access_key_id for profile {}", &profile_name);
                cfg.aws_temp_access_key_id = Some(aws_access_key);
            }
            if let Some(aws_secret_key) = profile_cfg
                .get("aws_secret_access_key")
                .and_then(|v| v.clone())
            {
                debug!("Found aws_secret_access_key for profile {}", &profile_name);
                cfg.aws_temp_secret_access_key = Some(aws_secret_key);
            }
            break;
        }
    }
    if aws_access_key.is_none() || aws_secret_key.is_none() {
        Err(anyhow!("No aws credentials found in {:?}", p))?;
    }

    cfg.aws_access_key_id = aws_access_key.ok_or(anyhow!("No aws_access_key found in {:?}", p))?;
    cfg.aws_secret_access_key =
        aws_secret_key.ok_or(anyhow!("No aws_secrete_key found in {:?}", p))?;

    Ok(())
}
