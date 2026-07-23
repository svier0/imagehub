use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub mime: String,
    pub extension: String,
    pub name: String,
    pub filename: String,
    pub size: String,
    pub width: String,
    pub height: String,
}
