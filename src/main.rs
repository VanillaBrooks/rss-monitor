// use futures::sync::mpsc;
// use futures::future::lazy;
// use futures::{Future, Stream, Sink};
use postgres::{Connection, TlsMode};
mod utils;
use rss_monitor::yaml;

struct DatabaseConfig {
    ip_address: String,
    port: u32,
    database_name: String,
    username: String,
    password: String,
}

impl DatabaseConfig {
    // fn new_serde() -> Self{
    // 	let path = r".\config.json".to_string();

    //     let file = fs::File::open(path).expect("config.json DOES NOT EXIST");
    //     let reader = io::BufReader::new(file);

    //     serde_json::de::from_reader(reader).expect("port, database, username, password were not all filled.")

    // }
    fn new(
        ip_address: &str,
        port: u32,
        database_name: &str,
        username: &str,
        password: &str,
    ) -> Self {
        DatabaseConfig {
            ip_address: ip_address.to_string(),
            port,
            database_name: database_name.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }
    fn connection_url(&self) -> String {
        format! {"postgresql://{}:{}@{}:{}/{}",self.username, self.password, self.ip_address, self.port, self.database_name}
    }
}

fn get_database_hashes() -> Result<postgres::Connection, postgres::Error> {
    let db = DatabaseConfig::new("localhost", 5432, "rss", "postgres", "pass");
    Connection::connect(db.connection_url(), TlsMode::None)
}

fn main() {
    let conn = get_database_hashes().expect("database connection error");

    // get hashes
    let days_old = 1;
    let now = utils::current_unix_time() as i64;
    let oldest = now - (days_old * 86400);
    let hashes = conn
        .query(
            "SELECT rss_hash FROM torrents WHERE insert_time > $1",
            &[&oldest],
        )
        .expect("bad query")
        .into_iter()
        .map(|x| {
            let x: i64 = x.get(0);
            x as u64
        })
        .collect::<std::collections::HashSet<_>>();

    let feed = yaml::FeedManager::from_yaml("config.yaml", hashes);
    dbg! {&feed};
    let mut feed = feed.expect("feed unwrap");
    feed.insert_trackers(&conn).expect("could not fetch trackers");

    loop {
        dbg!{"finished tracker iteration"};
        match feed.run_update(&conn) {
            Ok(next_update) => std::thread::sleep(std::time::Duration::from_secs(next_update as u64)),
            Err(err) => println!{"there was an error with updating the feeds: {:?} ", err}
        }

    }
}
