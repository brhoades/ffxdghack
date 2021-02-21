use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::collections::HashMap;

use regex::Regex;
use serde::{de::{Error as SerdeError}, Deserializer, Deserialize};
use anyhow::{Result, Error};
use url::Url;
use log::*;


#[derive(Debug, Deserialize)]
struct Config {
    patterns: Vec<Pattern>,
    profiles: HashMap<String, Option<std::path::PathBuf>>,
    default_profile: String,
}

#[derive(Debug, Deserialize)]
struct Pattern {
    #[serde(default, deserialize_with="new_regex_opt")]
    regex: Option<Regex>,
    #[serde(default, deserialize_with="new_regex_opt")]
    domain: Option<Regex>,
    #[serde(default, deserialize_with="new_regex_opt")]
    path: Option<Regex>,

    profile: String,
}

impl Pattern {
    fn matches(&self, url: &url::Url) -> bool {
        if let Some(regex) = &self.regex {
            if !regex.is_match(url.as_str()) {
                return false;
            }
        }

        if let Some(domain) = &self.domain {
            if url.domain().map(|d| !domain.is_match(d)).unwrap_or_default() {
                return false;
            }
        }

        if let Some(path) = &self.path {
            if !path.is_match(url.path()) {
                return false;
            }
        }

        true
    }
}

fn new_regex_opt<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Regex>, D::Error> {
    <Option<String>>::deserialize(deserializer)?
        .map(|re| Regex::new(&re))
        .transpose()
        .map_err(SerdeError::custom)
}

fn new_regex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Regex, D::Error> {
    String::deserialize(deserializer).and_then(|re| Regex::new(&re).map_err(SerdeError::custom))
}

fn tolerant_url(input: &String) -> Result<Url> {
    match Url::parse(input) {
        Ok(u) => Ok(u),
        Err(url::ParseError::RelativeUrlWithoutBase) => Url::parse(&format!("https://{}", input)).map_err(Error::from),
        Err(e) => Err(Error::from(e)),
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    debug!("start");

    let args: Vec<String> = env::args().skip(1).collect();
    let len = args.len();
    if args.len() < 2 {
        anyhow::bail!("insufficient arguments: need at least 2, got {}", args.len());
    }

    let cfg = args.first().unwrap();
    let url = tolerant_url(args.last().unwrap())?;
    let xargs = if len > 2 {
        args.get(1..(len-1)).unwrap().to_vec()
    } else {
        vec![]
    };

    debug!("config at {}, url is {}", cfg, url);
    let cfg: Config = serde_json::from_str(std::fs::read_to_string(cfg)?.as_str())?;

    let mut profile = cfg.default_profile;
    for pattern in cfg.patterns {
        if pattern.matches(&url) {
            info!("matched {:?}", pattern);
            profile = pattern.profile;
            break
        }
    }

    let mut cmd = Command::new("firefox");
    let mut cmd = cmd.args(xargs);
    let mut cmd = match cfg.profiles.get(&profile.to_string()) {
        Some(Some(path)) => {
            debug!("profile {} is at {}", profile, path.to_str().unwrap_or_default());
            cmd.arg("--profile").arg(path)
        },
        _ => {
            debug!("using named profile {}", profile);
            cmd.arg("-P").arg(profile)
        },
    };

    Err(Error::from(cmd.arg(url.as_str()).exec()))

}
