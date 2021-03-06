use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildConfig {
    root: String,
    name: String,
    bin_source: String,
    bin_target: String,
    java_source: String,
    java_target: String,
}

impl BuildConfig {
    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bin_source(&self) -> &str {
        &self.bin_source
    }

    pub fn bin_target(&self) -> &str {
        &self.bin_target
    }

    pub fn java_source(&self) -> &str {
        &self.java_source
    }

    pub fn java_target(&self) -> &str {
        &self.java_target
    }
}

#[derive(Debug, Default)]
pub struct BuildConfigBuilder {
    root: Option<String>,
    name: Option<String>,
    bin_source: Option<String>,
    bin_target: Option<String>,
    java_source: Option<String>,
    java_target: Option<String>,
}

impl BuildConfigBuilder {
    pub fn new() -> Self {
        BuildConfigBuilder {
            root: None,
            name: None,
            bin_source: None,
            bin_target: None,
            java_source: None,
            java_target: None,
        }
    }

    pub fn set_root<S>(mut self, root: S) -> BuildConfigBuilder
    where
        S: Into<String>,
    {
        self.root = Some(root.into());
        self
    }

    pub fn set_name<S>(mut self, name: S) -> BuildConfigBuilder
    where
        S: Into<String>,
    {
        self.name = Some(name.into());
        self
    }

    pub fn bin_source<S>(mut self, bin_source: S) -> BuildConfigBuilder
    where
        S: Into<String>,
    {
        self.bin_source = Some(bin_source.into());
        self
    }

    pub fn bin_target<S>(mut self, bin_target: S) -> BuildConfigBuilder
    where
        S: Into<String>,
    {
        self.bin_target = Some(bin_target.into());
        self
    }

    pub fn java_source<S>(mut self, java_source: S) -> BuildConfigBuilder
    where
        S: Into<String>,
    {
        self.java_source = Some(java_source.into());
        self
    }

    pub fn java_target<S>(mut self, java_target: S) -> BuildConfigBuilder
    where
        S: Into<String>,
    {
        self.java_target = Some(java_target.into());
        self
    }

    pub fn finish(self) -> BuildConfig {
        let root = self.root.unwrap_or_else(|| env::var("CARGO_MANIFEST_DIR").unwrap());
        let out_dir = env::var("OUT_DIR").unwrap();
        let default_bin_path = Path::new(&out_dir).join("../../../");
        let default_bin_source = default_bin_path.to_str().unwrap();
        BuildConfig {
            root: root.clone(),
            name: self.name.unwrap_or_else(|| env::var("CARGO_PKG_NAME").unwrap()),
            bin_source: self.bin_source.unwrap_or_else(|| default_bin_source.to_string()),
            bin_target: self
                .bin_target
                .unwrap_or_else(|| format!("{}/src/main/resources", root)),
            java_source: self
                .java_source
                .unwrap_or_else(|| format!("{}/java", env::var("OUT_DIR").unwrap())),
            java_target: self.java_target.unwrap_or_else(|| format!("{}/src/main", root)),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfigBuilder::new().finish()
    }
}

pub fn build(config: BuildConfig) {
    let encoded = serde_json::to_string_pretty(&config).expect("could not convert config");
    let path = format!("{}/roast.json", config.root);
    fs::write(path, encoded.as_bytes()).expect("could not write config");
}

pub fn config_from_path(path: &str) -> BuildConfig {
    let read = String::from_utf8(fs::read(path).unwrap()).unwrap();
    serde_json::from_str(&read).expect("could not decode build config")
}
