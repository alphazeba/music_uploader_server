# Music Uploader
this is the backend for https://github.com/alphazeba/music_uploader_gui.
Designed to be hosted on a server alongside plex.  Music Uploader allows your friends to easily contribute music to your plex server without technical knowhow or permissioned access to the server.


## Setup
#### Install rust: https://www.rust-lang.org/tools/install

#### you should use a webserver to handle ssl
the user passwords are sent via basic auth, you should only use music uploader behind https.
I use nginx, let's encrypt, & certbot to handle certs.

#### Configure Rocket.toml
copy Rocket.toml.example and renamed it Rocket.toml
- upload_dir should point to your plex music library
- max_mb determines how large of files in megabytes music_uploader will accept, increase or decrease at your will. 
    - (note: if using nginx or similar solution, it likely has a limiter which will need to be configured as well. nginx has client_max_body_size)
- plex_server_token you will need to get your server token to allow music uploader to trigger scans https://www.plexopedia.com/plex-media-server/general/plex-token/#plexservertoken
- plex_music_library you will need to find you music library key so that music uploader can target it for scanning 
    - example command for listing libraries `http://localhost:32400/library/sections?X-Plex-Token={{plexServerToken}}`
    - you are looking for the `key=` in the `<Directory>` component in the xml response related to your music library.
- valid_extensions defines what file extension music uploader will accept. If you would like to support other filetypes add their extension to the list.  You will also need to add the extension to your users' gui Settings.toml list.


#### Configure Secrets.toml
copy Secrets.toml.example and renamed it Secrets.toml
Remove demo users.
For each user create a section in the format.
```
[[users]]
username=""
password=""
```
This is storing your user's passwords in plaintext. So there is no pretense you should define and give your users' password to them.  If you would like to make this part better please open a pull request :)


# running
you will need rust installed.
You can run with using the default Rocket.toml settings.
```
cargo run
```

to use release overrides in Rocket.toml, run with
```
cargo run --release
```