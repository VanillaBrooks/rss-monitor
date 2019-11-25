use super::error::Error;
use super::rss::{self, TorrentData};
use super::utils;

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};

use reqwest;
use serde::{Deserialize, Serialize};
use serde_yaml;

#[derive(Debug, Deserialize)]
pub struct FeedManager {
    feeds: Vec<RssFeed>,
    #[serde(default)]
    next_update: u32,
    #[serde(skip)]
    client: Option<reqwest::Client>,
    #[serde(skip)]
    conn: Option<postgres::Connection>,

    // rss hashes that we have looked at
    #[serde(default)]
    previous_hashes: HashSet<u64>,
}

impl FeedManager {
    // Fetch yaml of configs to download
    pub fn from_yaml(path: &str, conn: postgres::Connection, days_old: i64) -> Result<FeedManager, Error> {
        let file = File::open(path)?;

        let now = utils::current_unix_time() as i64;
        let oldest = now - (days_old * 86400);
        let hashes = conn.query("SELECT rss_hash FROM torrents WHERE insert_time > $1", &[&oldest])
            .expect("bad query")
            .into_iter()
            .map(|x| {
                let x: i64 = x.get(0);
                x as u64
            })
            .collect::<HashSet<_>>();

        let mut yaml: FeedManager = serde_yaml::from_reader(file)?;
        yaml.client = Some(reqwest::Client::new());
        yaml.previous_hashes = hashes;
        yaml.conn = Some(conn);

        Ok(yaml)
    }

    // check all rss feeds for updates: update, pull torrents, and download them if possible
    pub fn run_update(&mut self) -> Result<u32, Error> {
        let mut next_update_time = 60 * 60;
        let epoch = utils::current_unix_time();

        let mut hashes_to_add = HashSet::new();

        // let prep = self.conn.prepare("INSERT INTO")

        let data = self.feeds
            .iter()
            .filter(|x| {

                // if the number of seconds since last update is greater than the number 
                // of seconds that we wait between updates we will update the RSS feed 
                if epoch - x.last_announce > x.update_interval {

                    // if the time to the next update is smaller than the current 
                    // greatest time to update we change the next update interval to
                    // correspond to this RSS feed
                    if x.update_interval < next_update_time {
                        next_update_time = x.update_interval
                    }

                    true

                // else: this RSS feed should not be updated yet
                } else {
                    false
                }
            })
            // for each RSS feed that needs updating, update it
            .map(|x| x.fetch_new(&self.client.as_ref().unwrap()))
            // if the rss parsing is Result::Ok()
            .filter(|x| x.is_ok())
            // unwrap good results
            .map(|x| x.unwrap())
            // flatten nested vectors to one vector
            .flatten()
            // send data to qbittorrent
            .map(|data| {
                // if we have not previously sent this to qbit...
                if !self.previous_hashes.contains(&data.item_hash) {
                    hashes_to_add.insert(data.item_hash);
                }
                data
            })
            .collect::<Vec<TorrentData>>();

        // insert current hashes into the list of hashes that do not need to be checked in the future
        hashes_to_add.into_iter().for_each(|hash| {
            self.previous_hashes.insert(hash);
        });

        self.next_update = next_update_time;

        Ok(next_update_time)
    }

}

#[derive(Debug, Deserialize)]
pub struct RssFeed {
    pub url: String,
    pub update_interval: u32,
    #[serde(default)]
    pub last_announce: u32,
    pub tracker: String
}
impl RssFeed {
    pub fn fetch_new(&self, pool: &reqwest::Client) -> Result<Vec<rss::TorrentData>, Error> {
        let mut response = pool.get(&self.url).send()?;
        let data = rss::xml_to_torrents(response)?;

        Ok(data)
    }
}
