use reqwest;
use serde_json as json;
use serde_xml_rs as xml;
use serde_yaml as yaml;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Serde(xml::Error),
    SerdeMissing,
    IoError(std::io::Error),
    YamlError(yaml::Error),
    JsonError(json::Error),
    SerdeGeneral,
}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}
impl From<xml::Error> for Error {
    fn from(e: xml::Error) -> Self {
        Error::Serde(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}
impl From<yaml::Error> for Error {
    fn from(e: yaml::Error) -> Self {
        Error::YamlError(e)
    }
}
impl From<json::Error> for Error {
    fn from(e: json::Error) -> Self {
        Error::JsonError(e)
    }
}
