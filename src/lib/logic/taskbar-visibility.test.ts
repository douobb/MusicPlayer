import { describe, expect, it } from 'vitest';
import { shouldShowTaskbarPlayer } from './taskbar-visibility';

describe('taskbar player visibility', () => {
  it('一般模式保持顯示', () => {
    expect(shouldShowTaskbarPlayer(false, true)).toBe(true);
  });

  it('預設在 Mini Player 模式暫時隱藏', () => {
    expect(shouldShowTaskbarPlayer(true, true)).toBe(false);
  });

  it('使用者可允許 Mini Player 與工作列播放器同時顯示', () => {
    expect(shouldShowTaskbarPlayer(true, false)).toBe(true);
  });
});
