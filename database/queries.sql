-- insering the tracker into the database
INSERT INTO tracker (tracker_name) VALUES ('tracker_name_here')
ON CONFLICT (tracker_id) 
DO NOTHING

-- insert torrent data into torrents
-- $1 torrent title
-- $2 downloaded bool
-- $3 rss hash i64
-- $4 time of insertion
-- $5 freeleech t/f
-- $6 size (potentiall null)
-- $7 evaluated (false)
-- $8 tracker name
with tracker_id_ as (
    SELECT tracker_id from tracker WHERE tracker_name=$8
)
INSERT INTO torrents 
(tracker_id, torrent_name, downloaded, rss_hash, insert_time, freeleech, size, evaluated) 
VALUES 
((SELECT * FROM tracker_id_), $1, $2, $3, $4, $5, $6, $7)
RETURNING torrent_id;
-- tracker_id                 torrent name    down hash time free size


-- insert tags
-- $1 tag_name
INSERT INTO tags (tag_name) VALUES ($1) ON CONFLICT (tag_name) DO NOTHING;

-- select all tags
-- $1 tag_name 
SELECT tag_id from tags where tag_name = $1;

-- insert tags into m:m table
-- $1: tag_id
-- $2: torrent_id
INSERT INTO tags_torrents (tag_id, torrent_id) VALUES ($1, $2)