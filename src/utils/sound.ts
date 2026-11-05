class SoundManager {
  private sounds: Record<string, HTMLAudioElement> = {};

  constructor() {
    if (typeof window !== 'undefined') {
      try {
        this.sounds.move = new Audio('/sounds/move.mp3');
        this.sounds.capture = new Audio('/sounds/capture.mp3');
        this.sounds.check = new Audio('/sounds/check.mp3');
        this.sounds.checkmate = new Audio('/sounds/checkmate.mp3');
      } catch (e) {
        console.warn('Audio initialization not supported:', e);
      }
    }
  }

  play(name: 'move' | 'capture' | 'check' | 'checkmate') {
    const sound = this.sounds[name];
    if (sound) {
      sound.currentTime = 0;
      sound.play().catch(() => {
        // Autoplay may be restricted before user interaction
      });
    }
  }
}

export const soundManager = new SoundManager();

