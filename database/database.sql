CREATE TABLE tracker (
    tracker_name VARCHAR(200) UNIQUE NOT NULL,
    tracker_id SERIAL PRIMARY KEY
);

CREATE TABLE torrents (
    torrent_id SERIAL PRIMARY KEY,
    tracker_id SERIAL REFERENCES tracker(tracker_id),
    torrent_name VARCHAR(200) NOT NULL,
    downloaded BOOLEAN NOT NULL,
    rss_hash BIGINT NOT NULL,
    insert_time BIGINT NOT NULL,
    evaluated BOOLEAN NOT NULL,
    freeleech BOOLEAN,
    size BIGINT
);

CREATE TABLE tags (
    tag_id SERIAL PRIMARY KEY,
    tag_name VARCHAR(30) UNIQUE NOT NULL
);

CREATE TABLE tags_torrents (
    tag_id SERIAL REFERENCES tags(tag_id) ON UPDATE CASCADE ON DELETE CASCADE,
    torrent_id SERIAL REFERENCES torrents(torrent_id) ON UPDATE CASCADE ON DELETE CASCADE,
    unique(tag_id, torrent_id)
);