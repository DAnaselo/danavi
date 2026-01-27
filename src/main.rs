mod audio;
mod client;
mod config;
mod tui;
mod types;

use anyhow::{Context, Result};
use audio::AudioPlayer;
use client::SubsonicClient;
use config::{config_needs_edit, get_config_path, load_config};
use std::io;
use std::time::Instant;
use tui::{Action, App, Tui};
use types::*;

const EASTER_EGG_PROBABILITY: f64 = 0.05;
const EASTER_EGGS: &[&str] = &[
    " - made with coffee",
    " - made with tea",
    " - made with something",
    " - made with cars",
    " - made with dry wall",
];

fn get_random_easter_egg(show_easter_eggs: bool) -> String {
    if !show_easter_eggs || rand::random::<f64>() > EASTER_EGG_PROBABILITY {
        return String::new();
    }
    let idx = rand::random::<usize>() % EASTER_EGGS.len();
    EASTER_EGGS[idx].to_string()
}

async fn load_artists(
    client: &SubsonicClient,
    app: &mut App,
    config: &types::Config,
) -> Result<()> {
    let response = client.get_artists().await?;
    app.artists = response
        .artists
        .index
        .into_iter()
        .flat_map(|idx| idx.artist)
        .map(|a| Artist {
            id: a.id,
            name: a.name,
        })
        .collect();

    let items: Vec<String> = app.artists.iter().map(|a| a.name.clone()).collect();
    app.set_items(items);
    app.current_base_content = format!("Artists{}", get_random_easter_egg(config.show_easter_eggs));
    Ok(())
}

async fn load_albums(
    client: &SubsonicClient,
    app: &mut App,
    artist_id: &str,
    config: &types::Config,
) -> Result<()> {
    let response = client.get_artist(artist_id).await?;
    app.albums = response
        .artist
        .album
        .into_iter()
        .map(|a| Album {
            id: a.id,
            name: a.name,
            artist_id: a.artist_id,
        })
        .collect();

    let items: Vec<String> = app.albums.iter().map(|a| a.name.clone()).collect();
    app.set_items(items);
    app.current_base_content = format!(
        "Albums for {}{}",
        response.artist.name,
        get_random_easter_egg(config.show_easter_eggs)
    );
    Ok(())
}

async fn load_songs(
    client: &SubsonicClient,
    app: &mut App,
    album_id: &str,
    config: &types::Config,
) -> Result<()> {
    let response = client.get_album(album_id).await?;
    app.songs = response
        .album
        .song
        .into_iter()
        .map(|s| Song {
            id: s.id,
            title: s.title,
            album_id: s.album_id,
        })
        .collect();

    let items: Vec<String> = app.songs.iter().map(|s| s.title.clone()).collect();
    app.set_items(items);
    app.current_base_content = format!(
        "Songs in {}{}",
        response.album.name,
        get_random_easter_egg(config.show_easter_eggs)
    );
    Ok(())
}

async fn handle_select(
    client: &SubsonicClient,
    app: &mut App,
    config: &types::Config,
    audio_player: &AudioPlayer,
) -> Result<()> {
    let selected = app.get_selected_index();
    if selected.is_none() {
        return Ok(());
    }
    let idx = selected.unwrap();

    match app.current_view {
        ViewType::Artists => {
            if let Some(artist) = app.artists.get(idx) {
                let artist_id = artist.id.clone();
                app.current_artist_id = Some(artist_id.clone());
                app.current_view = ViewType::Albums;
                load_albums(client, app, &artist_id, config).await?;
            }
        }
        ViewType::Albums => {
            if let Some(album) = app.albums.get(idx) {
                let album_id = album.id.clone();
                app.current_album_id = Some(album_id.clone());
                app.current_view = ViewType::Songs;
                load_songs(client, app, &album_id, config).await?;
            }
        }
        ViewType::Songs => {
            if let Some(song) = app.songs.get(idx) {
                let song = song.clone();
                play_song(client, app, song, audio_player).await?;
            }
        }
        ViewType::Search => {
            if let Some(result) = app.search_results.get(idx) {
                match result {
                    SearchResultItem::Album { id, artist_id, .. } => {
                        let id_clone = id.clone();
                        let artist_id_clone = artist_id.clone();
                        app.current_artist_id = Some(artist_id_clone);
                        app.current_album_id = Some(id_clone.clone());
                        app.current_view = ViewType::Songs;
                        load_songs(client, app, &id_clone, config).await?;
                    }
                    SearchResultItem::Song { id, title, .. } => {
                        let song = Song {
                            id: id.clone(),
                            title: title.clone(),
                            album_id: None,
                        };
                        play_song(client, app, song, audio_player).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn play_song(
    client: &SubsonicClient,
    app: &mut App,
    song: Song,
    audio_player: &AudioPlayer,
) -> Result<()> {
    app.show_message(format!("Playing: {}", song.title), 2000);

    let url = client.get_stream_url(&song.id);

    let response = reqwest::get(&url)
        .await
        .context("Failed to fetch audio stream")?;

    if !response.status().is_success() {
        anyhow::bail!("Server returned error: {}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read audio data")?
        .to_vec();

    audio_player
        .play_bytes(bytes)
        .context("Failed to play audio")?;

    // Note: Queue continuation would need to be handled through a callback or channel
    // For now, playback will stop when the song ends

    Ok(())
}

async fn play_next_in_queue(
    client: &SubsonicClient,
    app: &mut App,
    audio_player: &AudioPlayer,
) -> Result<()> {
    if !app.queue.is_empty() {
        let song = app.queue.remove(0);
        play_song(client, app, song, audio_player).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config().context("Failed to load config")?;

    if config_needs_edit(&config) {
        eprintln!("Config not found or using defaults!");
        eprintln!("Please edit the config file at: {:?}", get_config_path()?);
        eprintln!("Press Enter to continue anyway...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }

    let client = SubsonicClient::new(
        config.base_url.clone(),
        config.username.clone(),
        config.password.clone(),
    )?;

    // Initialize audio player
    let audio_player = AudioPlayer::new()
        .context("Failed to initialize audio player. Make sure audio output is available.")?;

    let mut app = App::new();
    let mut tui = Tui::new()?;

    // Initial load
    if let Err(e) = load_artists(&client, &mut app, &config).await {
        app.show_message(format!("Error: {}", e), 3000);
    }

    let mut last_message_check = Instant::now();

    loop {
        tui.draw(&mut app)?;

        // Check message timeout
        if let Some(timeout) = app.status_message_timeout {
            if last_message_check.elapsed().as_millis() as u64 >= timeout {
                app.clear_message();
                last_message_check = Instant::now();
            }
        } else {
            last_message_check = Instant::now();
        }

        if let Some(action) = tui.handle_event(&mut app)? {
            match action {
                tui::Action::Quit => break,
                Action::Select => {
                    if let Err(e) = handle_select(&client, &mut app, &config, &audio_player).await {
                        app.show_message(format!("Error: {}", e), 3000);
                    }
                }
                Action::AddToQueue => {
                    if let Some(idx) = app.get_selected_index() {
                        match app.current_view {
                            ViewType::Songs => {
                                if let Some(song) = app.songs.get(idx) {
                                    app.queue.push(song.clone());
                                    app.show_message(
                                        format!(
                                            "Added to queue: {} (Queue: {})",
                                            song.title,
                                            app.queue.len()
                                        ),
                                        1500,
                                    );
                                }
                            }
                            ViewType::Search => {
                                if let Some(result) = app.search_results.get(idx) {
                                    if let SearchResultItem::Song { id, title, .. } = result {
                                        app.queue.push(Song {
                                            id: id.clone(),
                                            title: title.clone(),
                                            album_id: None,
                                        });
                                        app.show_message(
                                            format!(
                                                "Added to queue: {} (Queue: {})",
                                                title,
                                                app.queue.len()
                                            ),
                                            1500,
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Action::PlayNext => {
                    if let Err(e) = play_next_in_queue(&client, &mut app, &audio_player).await {
                        app.show_message(format!("Error: {}", e), 3000);
                    }
                }
                Action::RestartQueue => {
                    if !app.queue.is_empty() {
                        // Stop current playback
                        audio_player.stop();
                        if let Err(e) = play_next_in_queue(&client, &mut app, &audio_player).await {
                            app.show_message(format!("Error: {}", e), 3000);
                        }
                    }
                }
                Action::TogglePause => {
                    audio_player.toggle_pause();
                    if audio_player.is_paused() {
                        app.show_message("Paused".to_string(), 1500);
                    } else {
                        app.show_message("Resumed".to_string(), 1500);
                    }
                }
                Action::Search => {
                    let query = app.search_string.clone();
                    app.current_view = ViewType::Search;
                    app.current_base_content = format!("Searching for \"{}\"...", query);
                    tui.draw(&mut app)?;

                    match client.search3(&query, 20, 20, 20).await {
                        Ok(response) => {
                            let mut items = Vec::new();
                            if let Some(search_result) = response.search_result3 {
                                if let Some(albums) = search_result.album {
                                    for album in albums {
                                        items.push(SearchResultItem::Album {
                                            id: album.id,
                                            name: album.name,
                                            artist: album.artist,
                                            artist_id: album.artist_id,
                                        });
                                    }
                                }
                                if let Some(songs) = search_result.song {
                                    for song in songs {
                                        items.push(SearchResultItem::Song {
                                            id: song.id,
                                            title: song.title,
                                            artist: song.artist,
                                            album_id: song.album_id,
                                        });
                                    }
                                }
                            }
                            app.search_results = items;
                            let search_items: Vec<String> = app
                                .search_results
                                .iter()
                                .map(|r| match r {
                                    SearchResultItem::Album { name, artist, .. } => {
                                        format!("[A] {} - {}", name, artist)
                                    }
                                    SearchResultItem::Song { title, artist, .. } => {
                                        format!("[S] {} - {}", title, artist)
                                    }
                                })
                                .collect();
                            app.set_items(search_items);
                            app.current_base_content = format!(
                                "Search: {} ({} results){}",
                                query,
                                app.search_results.len(),
                                get_random_easter_egg(config.show_easter_eggs)
                            );
                        }
                        Err(e) => {
                            app.show_message(format!("Search error: {}", e), 3000);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
