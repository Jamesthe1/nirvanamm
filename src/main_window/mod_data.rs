use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct ModMetaData {
    pub name: String,
    pub author: String,
    pub version: String
}

#[derive(Deserialize, Default)]
pub struct ModData {
    pub metadata: ModMetaData
}