export interface ArtistCredit {
  artist_id: number;
  name: string;
  position: number;
}

export type ArtistRole = 'performer' | 'original_performer';

export interface Track {
  id: number;
  file_path: string;
  title: string;
  performers: ArtistCredit[];
  original_performers: ArtistCredit[];
  duration_secs: number;
  cover_art: string | null;
  cover_art_path: string | null;
  file_size_bytes: number;
  play_count: number;
  last_played_at: string | null;
}

export interface LibraryFolder {
  id: number;
  folder_path: string;
  enabled: boolean;
  track_count: number;
  last_scan_at: string | null;
  last_error: string | null;
  last_added: number;
  last_updated: number;
  last_unchanged: number;
  last_removed: number;
  last_failed: number;
}

export interface FolderSyncResult {
  folder_id: number;
  added: number;
  updated: number;
  unchanged: number;
  removed: number;
  failed_files: FailedFile[];
}
export interface FailedFile {
  file_path: string;
  error: string;
}

export interface ImportResult {
  tracks: Track[];
  failed_files: FailedFile[];
}

export interface Playlist {
  id: number;
  name: string;
  track_ids: number[];
  last_position_track_id: number | null;
  last_position_secs: number | null;
  sort_order: number;
}

export interface TrackDetails {
  id: number;
  file_path: string;
  title: string;
  performers: ArtistCredit[];
  original_performers: ArtistCredit[];
  duration_secs: number;
  file_size_bytes: number;
  bitrate_kbps: number | null;
  sample_rate_hz: number | null;
  channels: number | null;
  format: string;
  bits_per_sample: number | null;
}

export type RepeatMode = 'off' | 'repeat-all' | 'repeat-one';

export interface PlayerState {
  is_playing: boolean;
  current_track_id: number | null;
  position_secs: number;
  duration_secs: number;
  volume: number;
  track_ended: boolean;
  gapless_queued: boolean;
  gapless_transitioned: boolean;
}

export type TaskbarMode = 'embedded' | 'docked' | 'unavailable';
export type TaskbarPreferenceMode = 'auto' | 'docked';

export interface TaskbarSettings {
  enabled: boolean;
  mode: TaskbarPreferenceMode;
  offset_x: number;
  show_title_marquee: boolean;
  show_progress: boolean;
  hide_in_mini_player: boolean;
}

export interface TaskbarStatus {
  supported: boolean;
  enabled: boolean;
  running: boolean;
  visible: boolean;
  mode: TaskbarMode | null;
  message: string;
}

export interface TaskbarSnapshot {
  title: string;
  artists: string;
  is_playing: boolean;
  volume: number;
  can_previous: boolean;
  can_next: boolean;
  position_secs: number;
  duration_secs: number;
  show_title_marquee: boolean;
  show_progress: boolean;
}

export interface ArtistSummary {
  id: number;
  name: string;
  track_count: number;
  performer_track_count: number;
  original_track_count: number;
}

export interface TagSummary {
  id: number;
  name: string;
  track_count: number;
}

export interface TagStatistics {
  tag_count: number;
  tagged_track_count: number;
  untagged_track_count: number;
  assignment_count: number;
  average_tags_per_tagged_track: number;
  most_used_tag: TagSummary | null;
}

export interface TagAssignment {
  id: number;
  name: string;
  assigned_count: number;
}

export type SortColumn = 'title' | 'artist' | 'duration_secs' | 'play_count';
export type SortDirection = 'asc' | 'desc';

export interface SortConfig {
  column: SortColumn;
  direction: SortDirection;
}
