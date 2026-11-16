export type Status = 'queued' | 'connecting' | 'downloading' | 'paused' | 'stopped' | 'completed' | 'failed';
export interface Download {
  id: string; url: string; name: string; directory: string; status: Status;
  downloaded: number; total: number | null; speed: number; connections: number;
  resumable: boolean; error: string | null; path: string | null; sha256: string | null;
  expected_sha256: string | null; resume_with_queue: boolean;
}
export interface Settings {
  directory: string; language: string; theme: string; compact: boolean;
  concurrent_downloads: number; connections_per_file: number; global_connections: number;
  host_connections: number; speed_limit: number; resume_on_launch: boolean; stop_on_error: boolean;
  schedule_enabled: boolean; schedule_start: number; schedule_end: number; close_to_tray: boolean;
}
export interface Snapshot { downloads: Download[]; settings: Settings; running: boolean; draining: boolean }
