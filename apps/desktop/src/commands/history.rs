use crate::actions::{process_transcription_output, OutputMode};
use crate::managers::{
    history::{EntryTexts, HistoryEntry, HistoryManager, PaginatedHistory},
    history_dictations::HistoryDictation,
    transcription::TranscriptionManager,
};
use fala_core::AppContext;
use fala_storage::{MetricsSummary, Percentiles, Showing};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<PaginatedHistory, String> {
    history_manager
        .get_history_entries(cursor, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_history_entry_saved(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .toggle_saved_status(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_audio_file_path(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_name: String,
) -> Result<String, String> {
    let path = history_manager.get_audio_file_path(&file_name);
    path.to_str()
        .ok_or_else(|| "Invalid file path".to_string())
        .map(|s| s.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .delete_entry(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_transcription(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    id: i64,
) -> Result<(), String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    let audio_path = history_manager.get_audio_file_path(&entry.file_name);
    let samples = crate::audio_toolkit::read_wav_samples(&audio_path)
        .map_err(|e| format!("Failed to load audio: {}", e))?;

    if samples.is_empty() {
        return Err("Recording has no audio samples".to_string());
    }

    transcription_manager.initiate_model_load();

    let tm = Arc::clone(&transcription_manager);
    let transcription = tauri::async_runtime::spawn_blocking(move || tm.transcribe(samples))
        .await
        .map_err(|e| format!("Transcription task panicked: {}", e))?
        .map_err(|e| e.to_string())?;

    if transcription.is_empty() {
        return Err("Recording contains no speech".to_string());
    }

    let mode = retry_mode(entry.post_process_requested, entry.dictation.as_ref());
    let processed = process_transcription_output(&app, &transcription, mode).await;
    history_manager
        .update_transcription(
            id,
            EntryTexts {
                transcription_text: transcription,
                post_processed_text: processed.post_processed_text,
                post_process_prompt: processed.post_process_prompt,
                pasted_text: processed.final_text,
                llm_produced: processed.llm_produced,
            },
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// How a retry formats: the legacy binding again, or the automatic path for the recorded app.
/// An entry without a dictation (the first transcription failed) never recorded its app, so
/// the LLM stays off rather than risk a disabled app (ADR-0004).
fn retry_mode(post_process_requested: bool, dictation: Option<&HistoryDictation>) -> OutputMode {
    match (post_process_requested, dictation) {
        (true, _) => OutputMode::Legacy,
        (false, Some(dictation)) => OutputMode::Auto(AppContext {
            app_name: dictation.app_name.clone(),
        }),
        (false, None) => OutputMode::RulesOnly,
    }
}

/// "Desfazer edição da IA": the entry's dictation shows its raw text again.
#[tauri::command]
#[specta::specta]
pub async fn undo_history_entry_edit(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<HistoryEntry, String> {
    history_manager
        .set_showing(id, Showing::Raw)
        .map_err(|e| e.to_string())
}

/// "Reaplicar edição da IA": the entry's dictation shows its edited text again.
#[tauri::command]
#[specta::specta]
pub async fn redo_history_entry_edit(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<HistoryEntry, String> {
    history_manager
        .set_showing(id, Showing::Final)
        .map_err(|e| e.to_string())
}

/// p50/p90 in ms; `None` without samples.
#[derive(Serialize, Type)]
pub struct LatencyPercentiles {
    pub p50: Option<u32>,
    pub p90: Option<u32>,
}

/// Dictations and words of one local day (`YYYY-MM-DD`).
#[derive(Serialize, Type)]
pub struct DictationDay {
    pub day: String,
    pub dictations: u32,
    pub words: u32,
}

/// `fala_storage::MetricsSummary` for the UI: `e2e` is release → pasted text without the LLM,
/// `e2e_llm` with it asked.
#[derive(Serialize, Type)]
pub struct DictationStats {
    pub days: u32,
    pub dictations: u32,
    pub words: u32,
    pub llm_attempts: u32,
    pub fallbacks: u32,
    pub per_day: Vec<DictationDay>,
    pub e2e: LatencyPercentiles,
    pub e2e_llm: LatencyPercentiles,
    pub asr: LatencyPercentiles,
    pub llm: LatencyPercentiles,
    pub paste: LatencyPercentiles,
    pub speech: LatencyPercentiles,
}

impl From<MetricsSummary> for DictationStats {
    fn from(s: MetricsSummary) -> Self {
        let p = |p: Percentiles| LatencyPercentiles {
            p50: p.p50,
            p90: p.p90,
        };
        Self {
            days: s.days,
            dictations: s.dictations,
            words: s.words,
            llm_attempts: s.llm_attempts,
            fallbacks: s.fallbacks,
            per_day: (s.per_day.into_iter())
                .map(|d| DictationDay {
                    day: d.day,
                    dictations: d.dictations,
                    words: d.words,
                })
                .collect(),
            e2e: p(s.e2e),
            e2e_llm: p(s.e2e_llm),
            asr: p(s.asr),
            llm: p(s.llm),
            paste: p(s.paste),
            speech: p(s.speech),
        }
    }
}

/// "Como estou indo": the last `days` days of dictation metrics.
#[tauri::command]
#[specta::specta]
pub async fn get_dictation_stats(
    history_manager: State<'_, Arc<HistoryManager>>,
    days: u32,
) -> Result<DictationStats, String> {
    history_manager
        .metrics_summary(days)
        .map(DictationStats::from)
        .map_err(|e| e.to_string())
}

/// Busca no histórico: os itens de `fala.sqlite` que casam com `query`, sem acento, mais
/// recentes primeiro, como linhas do histórico.
#[tauri::command]
#[specta::specta]
pub async fn history_search(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    query: String,
) -> Result<Vec<HistoryEntry>, String> {
    history_manager.search(&query).map_err(|e| e.to_string())
}

/// "Recuperar": a linha descartada volta ao normal, com o texto que tem.
#[tauri::command]
#[specta::specta]
pub async fn recover_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<HistoryEntry, String> {
    history_manager.recover(id).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_history_limit(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    limit: usize,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.history_limit = limit;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_recording_retention_period(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    period: String,
) -> Result<(), String> {
    use crate::settings::RecordingRetentionPeriod;

    let retention_period = match period.as_str() {
        "never" => RecordingRetentionPeriod::Never,
        "preserve_limit" => RecordingRetentionPeriod::PreserveLimit,
        "days3" => RecordingRetentionPeriod::Days3,
        "weeks2" => RecordingRetentionPeriod::Weeks2,
        "months3" => RecordingRetentionPeriod::Months3,
        _ => return Err(format!("Invalid retention period: {}", period)),
    };

    let mut settings = crate::settings::get_settings(&app);
    settings.recording_retention_period = retention_period;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::retry_mode;
    use crate::actions::OutputMode;
    use crate::managers::history_dictations::{HistoryDictation, HistoryEditor, HistoryShowing};
    use fala_core::AppContext;

    fn dictation(app_name: Option<&str>) -> HistoryDictation {
        HistoryDictation {
            raw_text: "oi".to_string(),
            final_text: "Oi.".to_string(),
            editor: HistoryEditor::Rules,
            showing: HistoryShowing::Final,
            app_name: app_name.map(str::to_string),
        }
    }

    #[test]
    fn retry_without_a_recorded_app_never_asks_the_llm() {
        assert_eq!(retry_mode(false, None), OutputMode::RulesOnly);
        assert_eq!(
            retry_mode(false, Some(&dictation(Some("keepassxc")))),
            OutputMode::Auto(AppContext {
                app_name: Some("keepassxc".to_string())
            })
        );
        assert_eq!(
            retry_mode(false, Some(&dictation(None))),
            OutputMode::Auto(AppContext::default())
        );
        assert_eq!(retry_mode(true, None), OutputMode::Legacy);
    }
}
