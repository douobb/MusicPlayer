export function shouldShowTaskbarPlayer(miniMode: boolean, hideInMiniPlayer: boolean): boolean {
  return !(miniMode && hideInMiniPlayer);
}
