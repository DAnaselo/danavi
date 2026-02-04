mod audio;
mod client;
mod config;
mod mpris;
mod tui;
mod types;

use anyhow::{Context, Result};
use audio::AudioPlayer;
use client::SubsonicClient;
use config::{config_needs_edit, get_config_path, load_config};
use mpris::{MprisCommand, MprisServer, PlaybackStatus};
use std::io;
use std::time::Instant;
use tokio::sync::mpsc;
use tui::{Action, App, Tui};
use types::*;

const EASTER_EGG_PROBABILITY: f64 = 0.05;
const EASTER_EGGS: &[&str] = &[
    " - made with coffee",
    " - made with tea",
    " - made with something",
    " - made with cars",
    " - made with dry wall",
    " - made with chocolate milk",
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
            artist: s.artist,
            album: Some(response.album.name.clone()),
            duration: s.duration,
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
    mpris_server: &MprisServer,
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
                // When playing from Songs view, set up album continuation
                let source = PlaybackSource::Album {
                    album_songs: app.songs.clone(),
                    current_index: idx,
                };
                play_song(client, app, song, audio_player, mpris_server, source).await?;
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
                    SearchResultItem::Song { id, title, artist, album_id, .. } => {
                        let song = Song {
                            id: id.clone(),
                            title: title.clone(),
                            album_id: Some(album_id.clone()),
                            artist: Some(artist.clone()),
                            album: None,
                            duration: None,
                        };
                        play_song(client, app, song, audio_player, mpris_server, PlaybackSource::Search).await?;
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
    mpris_server: &MprisServer,
    source: PlaybackSource,
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

    // Update MPRIS state and emit PropertiesChanged signal
    mpris_server.update_current_song(
        Some(mpris::Song {
            id: song.id.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            duration: song.duration,
        }),
        Some(url),
    ).await?;
    mpris_server.update_playback_status(PlaybackStatus::Playing).await?;

    // Track the playback source
    app.current_playback_source = Some(source);

    Ok(())
}

async fn play_next_in_queue(
    client: &SubsonicClient,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris_server: &MprisServer,
) -> Result<()> {
    if !app.queue.is_empty() {
        let song = app.queue.remove(0);
        play_song(client, app, song, audio_player, mpris_server, PlaybackSource::Queue).await?;
    } else {
        // No more songs in queue - update MPRIS state to stopped
        mpris_server.update_current_song(None, None).await?;
        mpris_server.update_playback_status(PlaybackStatus::Stopped).await?;
    }
    Ok(())
}

async fn play_next_in_album(
    client: &SubsonicClient,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris_server: &MprisServer,
    album_songs: &[Song],
    current_index: usize,
) -> Result<()> {
    let next_index = current_index + 1;
    if next_index < album_songs.len() {
        let next_song = album_songs[next_index].clone();
        play_song(
            client,
            app,
            next_song,
            audio_player,
            mpris_server,
            PlaybackSource::Album {
                album_songs: album_songs.to_vec(),
                current_index: next_index,
            },
        )
        .await?;
    } else {
        // Album finished - clear playback source and stop
        app.current_playback_source = None;
        mpris_server.update_current_song(None, None).await?;
        mpris_server.update_playback_status(PlaybackStatus::Stopped).await?;
    }
    Ok(())
}

async fn play_previous_in_album(
    client: &SubsonicClient,
    app: &mut App,
    audio_player: &AudioPlayer,
    mpris_server: &MprisServer,
    album_songs: &[Song],
    current_index: usize,
) -> Result<()> {
    if current_index > 0 {
        let prev_index = current_index - 1;
        let prev_song = album_songs[prev_index].clone();
        play_song(
            client,
            app,
            prev_song,
            audio_player,
            mpris_server,
            PlaybackSource::Album {
                album_songs: album_songs.to_vec(),
                current_index: prev_index,
            },
        )
        .await?;
    } else {
        // At first song - restart it
        let current_song = album_songs[current_index].clone();
        play_song(
            client,
            app,
            current_song,
            audio_player,
            mpris_server,
            PlaybackSource::Album {
                album_songs: album_songs.to_vec(),
                current_index,
            },
        )
        .await?;
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

    // Initialize MPRIS server
    let (mpris_command_tx, mut mpris_command_rx) = mpsc::unbounded_channel::<MprisCommand>();
    let (mpris_server, mpris_state) = MprisServer::new(mpris_command_tx).await?;
    
    // Set initial volume in MPRIS state
    mpris_server.update_volume(audio_player.get_volume()).await?;

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

        // Handle MPRIS commands
        while let Ok(command) = mpris_command_rx.try_recv() {
            match command {
                MprisCommand::Play => {
                    if audio_player.is_paused() {
                        audio_player.toggle_pause();
                        let _ = mpris_server.update_playback_status(PlaybackStatus::Playing).await;
                    } else if !app.queue.is_empty() {
                        let _ = play_next_in_queue(&client, &mut app, &audio_player, &mpris_server).await;
                    }
                }
                MprisCommand::Pause => {
                    if !audio_player.is_paused() && !audio_player.is_finished() {
                        audio_player.toggle_pause();
                        let _ = mpris_server.update_playback_status(PlaybackStatus::Paused).await;
                    }
                }
                MprisCommand::PlayPause => {
                    audio_player.toggle_pause();
                    if audio_player.is_paused() {
                        let _ = mpris_server.update_playback_status(PlaybackStatus::Paused).await;
                    } else if !audio_player.is_finished() {
                        let _ = mpris_server.update_playback_status(PlaybackStatus::Playing).await;
                    }
                }
                MprisCommand::Stop => {
                    audio_player.stop();
                    let _ = mpris_server.update_playback_status(PlaybackStatus::Stopped).await;
                }
                MprisCommand::Next => {
                    if !app.queue.is_empty() {
                        let _ = play_next_in_queue(&client, &mut app, &audio_player, &mpris_server).await;
                    } else if let Some(source) = app.current_playback_source.take() {
                        match source {
                            PlaybackSource::Album { album_songs, current_index } => {
                                let _ = play_next_in_album(&client, &mut app, &audio_player, &mpris_server, &album_songs, current_index).await;
                            }
                            _ => {
                                // For other sources, just stop
                                let _ = mpris_server.update_playback_status(PlaybackStatus::Stopped).await;
                            }
                        }
                    }
                }
                MprisCommand::Previous => {
                    if let Some(source) = app.current_playback_source.take() {
                        match source {
                            PlaybackSource::Album { album_songs, current_index } => {
                                let _ = play_previous_in_album(&client, &mut app, &audio_player, &mpris_server, &album_songs, current_index).await;
                            }
                            _ => {
                                // For other sources, restart current song if available
                                let state = mpris_state.read().await;
                                if let Some(current_song) = state.current_song.as_ref() {
                                    let song = Song {
                                        id: current_song.id.clone(),
                                        title: current_song.title.clone(),
                                        artist: current_song.artist.clone(),
                                        album: current_song.album.clone(),
                                        duration: current_song.duration,
                                        album_id: None,
                                    };
                                    drop(state);
                                    let _ = play_song(&client, &mut app, song, &audio_player, &mpris_server, source).await;
                                }
                            }
                        }
                    }
                }
                MprisCommand::SetVolume(volume) => {
                    let volume = volume.clamp(0.0, 1.0);
                    audio_player.set_volume(volume);
                    let _ = mpris_server.update_volume(volume).await;
                }
                _ => {}
            }
        }

        // Check if audio finished playing
        if !audio_player.is_paused() && audio_player.is_finished() {
            let state = mpris_state.read().await;
            if state.playback_status == PlaybackStatus::Playing {
                drop(state);
                if !app.queue.is_empty() {
                    // Queue takes priority - play next in queue
                    let _ = play_next_in_queue(&client, &mut app, &audio_player, &mpris_server).await;
                } else if let Some(source) = app.current_playback_source.take() {
                    // Check if we should continue based on playback source
                    match source {
                        PlaybackSource::Album { album_songs, current_index } => {
                            let _ = play_next_in_album(&client, &mut app, &audio_player, &mpris_server, &album_songs, current_index).await;
                        }
                        _ => {
                            // For Search, Queue (already handled), or Other sources - stop playback
                            let _ = mpris_server.update_playback_status(PlaybackStatus::Stopped).await;
                        }
                    }
                } else {
                    let _ = mpris_server.update_playback_status(PlaybackStatus::Stopped).await;
                }
            }
        }

        if let Some(action) = tui.handle_event(&mut app)? {
            match action {
                tui::Action::Quit => break,
                Action::Select => {
                    if let Err(e) = handle_select(&client, &mut app, &config, &audio_player, &mpris_server).await {
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
                                    if let SearchResultItem::Song { id, title, artist, album_id, .. } = result {
                                        app.queue.push(Song {
                                            id: id.clone(),
                                            title: title.clone(),
                                            album_id: Some(album_id.clone()),
                                            artist: Some(artist.clone()),
                                            album: None,
                                            duration: None,
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
                    if let Err(e) = play_next_in_queue(&client, &mut app, &audio_player, &mpris_server).await {
                        app.show_message(format!("Error: {}", e), 3000);
                    }
                }
                Action::RestartQueue => {
                    if !app.queue.is_empty() {
                        // Stop current playback
                        audio_player.stop();
                        if let Err(e) = play_next_in_queue(&client, &mut app, &audio_player, &mpris_server).await {
                            app.show_message(format!("Error: {}", e), 3000);
                        }
                    }
                }
                Action::TogglePause => {
                    audio_player.toggle_pause();
                    if audio_player.is_paused() {
                        app.show_message("Paused".to_string(), 1500);
                        let _ = mpris_server.update_playback_status(PlaybackStatus::Paused).await;
                    } else {
                        app.show_message("Resumed".to_string(), 1500);
                        let _ = mpris_server.update_playback_status(PlaybackStatus::Playing).await;
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
                                            album: song.album,
                                            duration: song.duration,
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
