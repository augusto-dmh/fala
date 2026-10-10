export interface ModelStateEvent {
  event_type: string;
  model_id?: string;
  model_name?: string;
  error?: string;
  /** A failure the UI translates instead of showing `error` (`model_not_found`, `model_not_downloaded`). */
  error_code?: string | null;
}

export interface RecordingErrorEvent {
  error_type: string;
  detail?: string;
}
