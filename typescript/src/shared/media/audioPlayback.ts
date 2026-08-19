import { create } from 'zustand';
import type { Message } from '@/shared/model/types';

interface AudioPlaybackState {
  message?: Message;
  isPlaying: boolean;
  progress: number;
  speed: number;
  play: (message: Message, source: string, playlist?: AudioPlaylistItem[]) => Promise<void>;
  pause: () => void;
  resume: () => Promise<void>;
  stop: () => void;
  toggleSpeed: () => void;
}

export interface AudioPlaylistItem {
  message: Message;
  source: string;
}

let player: HTMLAudioElement | undefined;

function audio(): HTMLAudioElement {
  if (player) return player;
  player = new Audio();
  player.preload = 'metadata';
  player.addEventListener('timeupdate', () => {
    useAudioPlayback.setState({ progress: player!.duration ? player!.currentTime / player!.duration : 0 });
  });
  player.addEventListener('play', () => useAudioPlayback.setState({ isPlaying: true }));
  player.addEventListener('pause', () => useAudioPlayback.setState({ isPlaying: false }));
  player.addEventListener('ended', () => useAudioPlayback.setState({ isPlaying: false, progress: 0 }));
  player.addEventListener('error', () => useAudioPlayback.setState({ isPlaying: false, progress: 0 }));
  return player;
}

export const useAudioPlayback = create<AudioPlaybackState>((set, get) => ({
  isPlaying: false,
  progress: 0,
  speed: 1,
  async play(message, source, playlist = []) {
    const current = get();
    const element = audio();
    if (current.message?.messageId === message.messageId) {
      await element.play();
      return;
    }
    await startPlayback(message, source, playlist, current.speed, set);
  },
  pause() { player?.pause(); },
  async resume() { if (player) await player.play(); },
  stop() {
    if (player) {
      player.pause();
      player.removeAttribute('src');
      player.load();
    }
    set({ message: undefined, isPlaying: false, progress: 0 });
  },
  toggleSpeed() {
    const speed = get().speed === 1 ? 2 : 1;
    if (player) player.playbackRate = speed;
    set({ speed });
  },
}));

async function startPlayback(message: Message, source: string, playlist: AudioPlaylistItem[], speed: number, set: (state: Partial<AudioPlaybackState>) => void) {
  const element = audio();
  element.pause();
  element.src = source;
  element.currentTime = 0;
  element.playbackRate = speed;
  const next = playlist.slice(playlist.findIndex((item) => item.message.messageId === message.messageId) + 1).find((item) => item.source);
  element.onended = next ? () => void startPlayback(next.message, next.source, playlist, speed, set) : null;
  set({ message, progress: 0, isPlaying: false });
  await element.play();
}
