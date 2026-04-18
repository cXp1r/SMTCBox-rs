use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtcMediaProperties {
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub album_artist: String,
    pub subtitle: String,
    pub track_number: u32,
    pub album_track_count: u32,
    pub genres: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtcPlaybackInfo {
    pub status: String,
    pub is_play_enabled: bool,
    pub is_pause_enabled: bool,
    pub is_stop_enabled: bool,
    pub is_next_enabled: bool,
    pub is_previous_enabled: bool,
    pub is_fast_forward_enabled: bool,
    pub is_rewind_enabled: bool,
    pub auto_repeat_mode: Option<String>,
    pub shuffle_active: Option<bool>,
    pub playback_rate: Option<f64>,
    pub playback_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtcTimelineProperties {
    pub start_time_secs: f64,
    pub end_time_secs: f64,
    pub position_secs: f64,
    pub min_seek_time_secs: f64,
    pub max_seek_time_secs: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtcSession {
    pub source_app_user_model_id: String,
    pub is_current: bool,
    pub media_properties: Option<SmtcMediaProperties>,
    pub playback_info: Option<SmtcPlaybackInfo>,
    pub timeline_properties: Option<SmtcTimelineProperties>,
}

#[tauri::command]
async fn get_smtc_sessions() -> Result<Vec<SmtcSession>, String> {
    tauri::async_runtime::spawn_blocking(|| fetch_sessions().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(target_os = "windows")]
fn fetch_sessions() -> windows::core::Result<Vec<SmtcSession>> {
    use windows::Foundation::Collections::IVectorView;
    use windows::Foundation::{AsyncStatus, IReference, TimeSpan};
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionMediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Media::{MediaPlaybackAutoRepeatMode, MediaPlaybackType};
    use windows::core::HSTRING;

    fn ts_secs(ts: TimeSpan) -> f64 {
        ts.Duration as f64 / 10_000_000.0
    }

    // --- Acquire session manager (inline spin-wait, no generic helper) ---
    let manager_op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?;
    loop {
        if manager_op.Status()? != AsyncStatus::Started {
            break;
        }
        std::hint::spin_loop();
    }
    let manager: GlobalSystemMediaTransportControlsSessionManager = manager_op.GetResults()?;

    let sessions: IVectorView<GlobalSystemMediaTransportControlsSession> = manager.GetSessions()?;

    let current_id: Option<String> = if let Ok(cs) = manager.GetCurrentSession() {
        cs.SourceAppUserModelId().ok().map(|s: HSTRING| s.to_string())
    } else {
        None
    };

    let count = sessions.Size()?;
    let mut result = Vec::new();

    for i in 0..count {
        let session: GlobalSystemMediaTransportControlsSession = sessions.GetAt(i)?;

        let source_id: String = session.SourceAppUserModelId().unwrap_or_default().to_string();
        let is_current = current_id.as_deref() == Some(source_id.as_str());

        // --- Media Properties (inline spin-wait) ---
        let media_properties: Option<SmtcMediaProperties> =
            if let Ok(props_op) = session.TryGetMediaPropertiesAsync() {
                loop {
                    match props_op.Status() {
                        Ok(s) if s != AsyncStatus::Started => break,
                        Err(_) => break,
                        _ => std::hint::spin_loop(),
                    }
                }
                let props: Option<GlobalSystemMediaTransportControlsSessionMediaProperties> =
                    props_op.GetResults().ok();
                if let Some(props) = props {
                    let genres: Vec<String> = match props.Genres() {
                        Ok(g) => {
                            let g: IVectorView<HSTRING> = g;
                            let n = g.Size().unwrap_or(0);
                            (0..n)
                                .filter_map(|j| g.GetAt(j).ok().map(|s: HSTRING| s.to_string()))
                                .collect()
                        }
                        Err(_) => vec![],
                    };
                    Some(SmtcMediaProperties {
                        title: props.Title().unwrap_or_default().to_string(),
                        artist: props.Artist().unwrap_or_default().to_string(),
                        album_title: props.AlbumTitle().unwrap_or_default().to_string(),
                        album_artist: props.AlbumArtist().unwrap_or_default().to_string(),
                        subtitle: props.Subtitle().unwrap_or_default().to_string(),
                        track_number: props.TrackNumber().unwrap_or(0) as u32,
                        album_track_count: props.AlbumTrackCount().unwrap_or(0) as u32,
                        genres,
                    })
                } else {
                    None
                }
            } else {
                None
            };

        // --- Playback Info ---
        let playback_info: Option<SmtcPlaybackInfo> = if let Ok(info) = session.GetPlaybackInfo() {
            let status: String = if let Ok(s) = info.PlaybackStatus() {
                match s {
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "Playing",
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "Paused",
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => "Stopped",
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Opened => "Opened",
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing => "Changing",
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => "Closed",
                    _ => "Unknown",
                }
            } else {
                "Unknown"
            }
            .to_string();

            let (
                is_play_enabled,
                is_pause_enabled,
                is_stop_enabled,
                is_next_enabled,
                is_previous_enabled,
                is_fast_forward_enabled,
                is_rewind_enabled,
            ) = if let Ok(ctrl) = info.Controls() {
                (
                    ctrl.IsPlayEnabled().unwrap_or(false),
                    ctrl.IsPauseEnabled().unwrap_or(false),
                    ctrl.IsStopEnabled().unwrap_or(false),
                    ctrl.IsNextEnabled().unwrap_or(false),
                    ctrl.IsPreviousEnabled().unwrap_or(false),
                    ctrl.IsFastForwardEnabled().unwrap_or(false),
                    ctrl.IsRewindEnabled().unwrap_or(false),
                )
            } else {
                (false, false, false, false, false, false, false)
            };

            let auto_repeat_mode: Option<String> = info
                .AutoRepeatMode()
                .ok()
                .and_then(|r: IReference<MediaPlaybackAutoRepeatMode>| r.Value().ok())
                .map(|arm| {
                    match arm {
                        MediaPlaybackAutoRepeatMode::None => "None",
                        MediaPlaybackAutoRepeatMode::Track => "Track",
                        MediaPlaybackAutoRepeatMode::List => "List",
                        _ => "Unknown",
                    }
                    .to_string()
                });

            let playback_type: Option<String> = info
                .PlaybackType()
                .ok()
                .and_then(|r: IReference<MediaPlaybackType>| r.Value().ok())
                .map(|pt| {
                    match pt {
                        MediaPlaybackType::Unknown => "Unknown",
                        MediaPlaybackType::Music => "Music",
                        MediaPlaybackType::Video => "Video",
                        MediaPlaybackType::Image => "Image",
                        _ => "Other",
                    }
                    .to_string()
                });

            let shuffle_active: Option<bool> = info
                .IsShuffleActive()
                .ok()
                .and_then(|r: IReference<bool>| r.Value().ok());

            let playback_rate: Option<f64> = info
                .PlaybackRate()
                .ok()
                .and_then(|r: IReference<f64>| r.Value().ok());

            Some(SmtcPlaybackInfo {
                status,
                is_play_enabled,
                is_pause_enabled,
                is_stop_enabled,
                is_next_enabled,
                is_previous_enabled,
                is_fast_forward_enabled,
                is_rewind_enabled,
                auto_repeat_mode,
                shuffle_active,
                playback_rate,
                playback_type,
            })
        } else {
            None
        };

        // --- Timeline Properties ---
        let timeline_properties: Option<SmtcTimelineProperties> =
            if let Ok(tl) = session.GetTimelineProperties() {
                Some(SmtcTimelineProperties {
                    start_time_secs: tl.StartTime().map(ts_secs).unwrap_or(0.0),
                    end_time_secs: tl.EndTime().map(ts_secs).unwrap_or(0.0),
                    position_secs: tl.Position().map(ts_secs).unwrap_or(0.0),
                    min_seek_time_secs: tl.MinSeekTime().map(ts_secs).unwrap_or(0.0),
                    max_seek_time_secs: tl.MaxSeekTime().map(ts_secs).unwrap_or(0.0),
                })
            } else {
                None
            };

        result.push(SmtcSession {
            source_app_user_model_id: source_id,
            is_current,
            media_properties,
            playback_info,
            timeline_properties,
        });
    }

    Ok(result)
}

#[cfg(not(target_os = "windows"))]
fn fetch_sessions() -> Result<Vec<SmtcSession>, String> {
    Ok(vec![])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_smtc_sessions])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
