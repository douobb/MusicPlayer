import type { Playlist } from '$lib/types';

type ActiveView =
  | { kind: 'library' }
  | { kind: 'playlist'; playlistId: number }
  | { kind: 'artists' }
  | { kind: 'artist-detail'; artistId: number; artistName: string }
  | { kind: 'tags' }
  | { kind: 'tag-detail'; tagId: number; tagName: string }
  | { kind: 'most-played' }
  | { kind: 'settings' };

let playlists = $state<Playlist[]>([]);
let activeView = $state<ActiveView>({ kind: 'library' });

export function getPlaylistState() {
  return {
    get playlists() {
      return playlists;
    },
    set playlists(v: Playlist[]) {
      playlists = v;
    },
    get activeView() {
      return activeView;
    },
    set activeView(v: ActiveView) {
      activeView = v;
    },
  };
}
