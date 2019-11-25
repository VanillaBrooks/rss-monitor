// use futures::sync::mpsc;
// use futures::future::lazy;
// use futures::{Future, Stream, Sink};
use postgres::{Connection, TlsMode};
mod utils;
use rss_monitor::yaml;

struct DatabaseConfig{
    ip_address: String,
	port: u32,
	database_name: String,
	username: String,
	password: String
}

impl DatabaseConfig {
	// fn new_serde() -> Self{
	// 	let path = r".\config.json".to_string();

    //     let file = fs::File::open(path).expect("config.json DOES NOT EXIST");
    //     let reader = io::BufReader::new(file);

    //     serde_json::de::from_reader(reader).expect("port, database, username, password were not all filled.")
		
    // }
    fn new(ip_address: &str, port: u32, database_name: &str, username: &str, password: &str)-> Self {
        DatabaseConfig {
            ip_address: ip_address.to_string(),
            port,
            database_name: database_name.to_string(),
            username: username.to_string(),
            password: password.to_string()
        }
    }
    fn connection_url(&self) -> String {
        format!{"postgresql://{}:{}@{}:{}/{}",self.username, self.password, self.ip_address, self.port, self.database_name}
    }
}


fn get_database_hashes() -> Result<postgres::Connection, postgres::Error> {
    let db = DatabaseConfig::new("localhost", 5432, "rss", "postgres", "pass");
    Connection::connect(db.connection_url(), TlsMode::None)
}

fn main() {
    let conn = get_database_hashes().expect("database connection error");
    let feed = yaml::FeedManager::from_yaml("config.yaml", conn, 1);
    dbg!{&feed};
}
