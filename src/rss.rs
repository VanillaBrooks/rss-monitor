use super::error::Error;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
// custom RSS parsing for non-standard rss feeds
use serde_xml_rs as xml;

#[derive(Deserialize, Debug)]
struct Document {
    channel: Option<Channel>,
}

#[derive(Deserialize, Debug)]
struct Channel {
    title: Option<String>,
    description: Option<String>,
    item: Option<Vec<Item>>,
}

#[derive(Deserialize, Debug)]
struct Item {
    title: Option<String>,
    link: Option<String>,
    tags: Option<String>,
    torrent: Option<Torrent>,
    enclosure: Option<Enclosure>,
    description: Option<String>,
}
impl std::hash::Hash for Item {
    fn hash<T: std::hash::Hasher>(&self, state: &mut T) {
        self.title.hash(state);
        self.tags.hash(state);
        self.description.hash(state);
    }
}

#[derive(Deserialize, Debug, Hash)]
struct Torrent {
    #[serde(rename="fileName")]
    file_name: Option<String>,
    #[serde(rename="infoHash")]
    info_hash: Option<String>,
    #[serde(rename="contentLength")]
    content_length: Option<u64>,
}

#[derive(Deserialize, Debug, Hash)]
struct Enclosure {
    url: Option<String>,
}

impl Item {
    fn link(&self) -> Result<String, Error> {
        if let Some(enclosure) = &self.enclosure {
            if let Some(url) = &enclosure.url {
                return Ok(url.clone());
            }
        }

        if let Some(link) = &self.link {
            return Ok(link.clone());
        }

        Err(Error::SerdeMissing)
    }
}

impl Torrent {
    fn default() -> Self {
        Self {
            file_name: None,
            info_hash: None,
            content_length: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TorrentData {
    pub title: String,
    pub tags: HashSet<String>,
    pub download_link: String,
    pub size: Option<u64>,
    pub item_hash: u64,
}
impl TorrentData {
    fn new(mut item: Item) -> Result<Self, Error> {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let hash = hasher.finish();

        let link = item.link()?;

        let tags = match &item.tags {
            Some(tags) => tags
                .split(' ')
                .map(|x| x.to_string().to_lowercase())
                .collect(),
            None => HashSet::new(),
        };
        let torrent = match item.torrent {
            Some(torrent) => torrent,
            None => Torrent::default(),
        };

        Ok(Self {
            title: item.title.take().unwrap().to_lowercase(),
            tags,
            download_link: link,
            size: torrent.content_length,
            item_hash: hash,
        })
    }
    pub fn postgres_size(&self) -> Option<i64> {
        match self.size {
            Some(good_size) => Some(good_size as i64),
            None => None,
        }
    }
}

pub fn xml_to_torrents<T: std::io::Read>(data: T) -> Result<Vec<TorrentData>, Error> {
    let doc: Document = xml::from_reader(data)?;

    if let Some(channel) = doc.channel {
        if let Some(items) = channel.item {
            let t_data = items
                .into_iter()
                .map(TorrentData::new) 
                .filter(|item| item.is_ok())
                .map(|item| item.unwrap())
                .collect::<Vec<_>>();

            Ok(t_data)
        } else {
            Err(Error::SerdeMissing)
        }
    } else {
        Err(Error::SerdeMissing)
    }
}
