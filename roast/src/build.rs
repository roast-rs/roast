use serde_json;
use std::env;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildConfig {
    pub root: String,
    pub name: String,
    pub bin_source: String,
    pub bin_target: String,
    pub java_source: String,
    pub java_target: String,
}

pub fn build() {
    let rootdir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let config = BuildConfig {
        root: rootdir.clone(),
        name: env::var("CARGO_PKG_NAME").unwrap(),
        bin_source: format!("target/{}", env::var("PROFILE").unwrap()),
        bin_target: "src/main/resources".into(),
        java_source: format!("{}/java", env::var("OUT_DIR").unwrap()),
        java_target: "src/main".into(),
    };

    let encoded = serde_json::to_string(&config).expect("could not convert config");
    let path = format!("{}/roast.json", rootdir);
    fs::write(path, encoded.as_bytes()).expect("could not write config");
}

pub fn config_from_path(path: &str) -> BuildConfig {
    let read = String::from_utf8(fs::read(path).unwrap()).unwrap();
    serde_json::from_str(&read).expect("could not decode build config")
}
