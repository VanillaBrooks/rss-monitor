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

    // rss hashes that we have looked at
    #[serde(default)]
    previous_hashes: HashSet<u64>,
}

impl FeedManager {
    // Fetch yaml of configs to download
    pub fn from_yaml(path: &str, hashes: HashSet<u64>) -> Result<FeedManager, Error> {
        let file = File::open(path)?;

        let mut yaml: FeedManager = serde_yaml::from_reader(file)?;
        yaml.client = Some(reqwest::Client::new());
        yaml.previous_hashes = hashes;

        Ok(yaml)
    }

    pub fn insert_trackers(&self, conn: &postgres::Connection) -> Result<(), Error> {
        let ins_tracker = conn
            .prepare(
                "
        INSERT INTO tracker (tracker_name) VALUES ($1)
        ON CONFLICT (tracker_id) 
        DO NOTHING",
            )
            .expect("insertion tracker err");

        self.feeds.iter().for_each(|rss_feed| {
            let tracker_name = &rss_feed.tracker;
            ins_tracker.execute(&[tracker_name]);
        });

        Ok(())
    }

    // check all rss feeds for updates: update, pull torrents, and download them if possible
    pub fn run_update(&mut self, conn: &postgres::Connection) -> Result<u32, Error> {
        let mut next_update_time = 60 * 60;
        let epoch = utils::current_unix_time();

        let ins_torrents = conn.prepare("
            with tracker_id_ as (
                SELECT tracker_id from tracker WHERE tracker_name=$8
            )
            INSERT INTO torrents 
            (tracker_id, torrent_name, downloaded, rss_hash, insert_time, freeleech, size, evaluated) 
            VALUES 
            ((SELECT * FROM tracker_id_), $1, $2, $3, $4, $5, $6, $7)
            RETURNING torrent_id;").expect("ins torrent query");

        let ins_tags = conn
            .prepare(
                "
            INSERT INTO tags (tag_name) VALUES ($1) ON CONFLICT (tag_name) DO NOTHING;
        ",
            )
            .expect("ins tags ");

        let sel_tags = conn
            .prepare(
                "
            SELECT tag_id from tags where tag_name = $1;
        ",
            )
            .expect("sel tags");

        let ins_tag_torrents = conn
            .prepare(
                "
            INSERT INTO tags_torrents (tag_id, torrent_id) VALUES ($1, $2)
        ",
            )
            .expect("ins tags torrents");

        let now = utils::current_unix_time() as i64;

        // dbg! {"running through main loop"};
        // dbg! {&self.feeds};

        let hashes_to_add = 
            self.feeds
                .iter()
                .filter(|x| {
                    // dbg! {"in main loop"};
                    // if the number of seconds since last update is greater than the number
                    // of seconds that we wait between updates we will update the RSS feed
                    if epoch - x.last_announce > x.update_interval {
                        // if the time to the next update is smaller than the current
                        // greatest time to update we change the next update interval to
                        // correspond to this RSS feed
                        if x.update_interval < next_update_time {
                            next_update_time = x.update_interval
                        }
                        // dbg! {"going to run update"};
                        true

                    // else: this RSS feed should not be updated yet
                    } else {
                        // dbg! {"shouldnt update"};
                        false
                    }
                })
                .map(|x| self.update_tracker(
                    x, 
                    &self.client.as_ref().unwrap(), 
                    &ins_torrents,
                    &ins_tags,
                    &sel_tags,
                    &ins_tag_torrents,
                ))
                .filter(|x| x.is_ok())
                .map(|x| x.unwrap())
                .flatten()
                .collect::<HashSet<u64>>();

        // insert current hashes into the list of hashes that do not need to be checked in the future
        hashes_to_add.into_iter().for_each(|hash| {
            self.previous_hashes.insert(hash);
        });

        self.next_update = next_update_time;

        Ok(next_update_time)
    }

    fn update_tracker(
        &self, 
        feed: &RssFeed, 
        client: &reqwest::Client, 
        ins_torrents: &postgres::stmt::Statement<'_>,
        ins_tags: &postgres::stmt::Statement<'_>,
        sel_tags: &postgres::stmt::Statement<'_>,
        ins_tag_torrents: &postgres::stmt::Statement<'_>,
    ) -> Result<HashSet<u64>, Error> {

        let data = feed.fetch_new(&client)?;
        let mut hashes_to_add = HashSet::with_capacity(10);

        let now = utils::current_unix_time() as i64;

        data.iter()
            .filter(|data| {
                if !self.previous_hashes.contains(&data.item_hash) {
                    hashes_to_add.insert(data.item_hash);
                    // dbg! {true};
                    true
                } else {
                    // dbg! {false};
                    false
                }
            })
            .map(|torrent| {
                // insert torrents
                let torrent_id = ins_torrents.query(&[
                    &torrent.title,              // 1
                    &false,                      // 2
                    &(torrent.item_hash as i64), // 3
                    &now,                        // 4
                    &false,                      // 5
                    &torrent.postgres_size(),    // 6
                    &false,                      // 7
                    &feed.tracker
                ]);

                // dbg! {&torrent_id};

                (torrent, torrent_id)
            })
            .filter(|(_, t_res)| t_res.is_ok())
            .map(|(x, t_res)| {
                let row = t_res.unwrap();
                // dbg!{&row};
                let row = row.get(0);
                // dbg!{&row};
                let val: Option<Result<i32, _>> = row.get_opt(0);
                // dbg!{&val};
                (x, val)
            })
            // get out of the option
            .filter(|(_, torrent_id_opt)| {
                if let Some((Ok(_))) = torrent_id_opt {
                    true
                }
                else {
                    false
                }
            })
            .for_each(|(torrent, torrent_id_res)| {
                let torrent_id = torrent_id_res.unwrap().unwrap();

                // insert each tag and select its id
                let tag_ids = torrent
                    .tags
                    .iter()
                    .map(|tag| {
                        let a = ins_tags.execute(&[tag]);
                        // dbg!{&a};
                        tag
                    })
                    .map(|tag| sel_tags.query(&[tag]))
                    .filter(|x| x.is_ok())
                    .map(|rows| {
                        let rows = rows.unwrap();

                        let row = rows.get(0);
                        let val: Option<Result<i32, _>> = row.get_opt(0);
                        val
                    })
                    .filter(|x| if let Some(Ok(x)) = x { true } else { false })
                    .map(|x| {
                        let tag_id = x.unwrap().unwrap();
                        tag_id
                    })
                    .collect::<Vec<_>>();

                // update many to many table
                tag_ids.iter().for_each(|tag_id| {
                    ins_tag_torrents.execute(&[tag_id, &torrent_id]);
                    ()
                });

            });

        /*

        // for each RSS feed that needs updating, update it
        .map(|rss| ( rss.tracker, rss.fetch_new(&self.client.as_ref().unwrap()) ) )
        // if the rss parsing is Result::Ok()
        .filter(|fetch| fetch.is_ok())
        // unwrap good results
        .map(|fetch| (tracker, fetch.unwrap()))
        // flatten nested vectors to one vector
        // send data to qbittorrent
        .map(|(tracker, data_vec)| {
            dbg!{"made past first filter"};
            // if we have not previously sent this to qbit...
            data_vec.iter().map(|data|{
            }).filter(|x| x);

        })

        */

        Ok(hashes_to_add)
    }
}

#[derive(Debug, Deserialize)]
pub struct RssFeed {
    pub url: String,
    pub update_interval: u32,
    #[serde(default)]
    pub last_announce: u32,
    pub tracker: String,
}
impl RssFeed {
    pub fn fetch_new(&self, pool: &reqwest::Client) -> Result<Vec<rss::TorrentData>, Error> {
        let mut response = pool.get(&self.url).send()?;
        let data = rss::xml_to_torrents(response)?;

        Ok(data)
    }
}
