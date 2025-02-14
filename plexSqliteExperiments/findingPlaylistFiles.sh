# open sqlite table
sqlite3 '/var/lib/plexmediaserver/Library/Application Support/Plex Media Server/Plug-in Support/Databases/com.plexapp.plugins.library.db'

# list the playlists
select title from metadata_items where metadata_type = 15;

select * from metadata_items where metadata_type = 15;

# list the columns of the 
PRAGMA table_info(metadata_items);


# list a user's playlists.


# stalk seth's playlist
select file 
  from media_parts # has the filename
  left outer join media_items on # links file name to metata
  media_items.id = media_parts.media_item_id 
  left outer join play_queue_generators # contains the playlist id
  on play_queue_generators.metadata_item_id = media_items.metadata_item_id 
  left outer join metadata_items on # contains the playlist information
  metadata_items.id = play_queue_generators.playlist_id
  where metadata_items.title = 'Recently Played';

select file 
  from media_parts
  left outer join media_items on
  media_items.id = media_parts.media_item_id 
  left outer join play_queue_generators
  on play_queue_generators.metadata_item_id = media_items.metadata_item_id 
  left outer join metadata_items on
  metadata_items.id = play_queue_generators.playlist_id
  where metadata_items.title = 'KARAOKE';

# try hunting things\
PRAGMA table_info(media_parts);
PRAGMA table_info(media_items);
select * from media_parts left outer join media_items on
  media_items.id = media_parts.media_item_id

PRAGMA table_info(play_queue_generators);