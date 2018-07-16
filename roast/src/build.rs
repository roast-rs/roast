use serde_json;
use std::env;
use std::fs;

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

#[derive(Debug)]
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
        BuildConfig {
            root: self.root.unwrap_or(env::var("CARGO_MANIFEST_DIR").unwrap()),
            name: self.name.unwrap_or(env::var("CARGO_PKG_NAME").unwrap()),
            bin_source: self.bin_source
                .unwrap_or(format!("target/{}", env::var("PROFILE").unwrap())),
            bin_target: self.bin_target.unwrap_or("src/main/resources".into()),
            java_source: self.java_source
                .unwrap_or(format!("{}/java", env::var("OUT_DIR").unwrap())),
            java_target: self.java_target.unwrap_or("src/main".into()),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfigBuilder::new().finish()
    }
}

pub fn build(config: BuildConfig) {
    let encoded = serde_json::to_string(&config).expect("could not convert config");
    let path = format!("{}/roast.json", config.root);
    fs::write(path, encoded.as_bytes()).expect("could not write config");
}

pub fn config_from_path(path: &str) -> BuildConfig {
    let read = String::from_utf8(fs::read(path).unwrap()).unwrap();
    serde_json::from_str(&read).expect("could not decode build config")
}
