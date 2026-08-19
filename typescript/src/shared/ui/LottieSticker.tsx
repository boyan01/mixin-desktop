import type { AnimationItem, LottiePlayer } from 'lottie-web';
import { useEffect, useRef, useState } from 'react';
import { e } from '@/shared/lib/logger';
import { evictCachedResource, loadCachedResource } from '@/shared/lib/networkResourceCache';
import { classes } from '@/shared/styles/classes';

const lottieDataCache = new Map<string, Promise<Record<string, unknown>>>();
let lottiePlayerPromise: Promise<LottiePlayer> | undefined;

function loadLottiePlayer() {
  lottiePlayerPromise ??= import('lottie-web').then((module) => module.default);
  return lottiePlayerPromise;
}

/**
 * Flutter's CacheClient keeps decoded Lottie payloads reusable across sticker
 * cells. Keep that ownership at module scope, while every mounted cell owns
 * and destroys its own renderer instance.
 */
export function loadLottieData(source: string, refresh = false) {
  if (refresh) lottieDataCache.delete(source);
  const cached = refresh ? undefined : lottieDataCache.get(source);
  if (cached) return cached;
  const request = (async () => {
    const { bytes } = await loadCachedResource(source, 'lottie', refresh);
    return JSON.parse(new TextDecoder().decode(bytes)) as Record<string, unknown>;
  })();
  lottieDataCache.set(source, request);
  // Do not pin a transient network or decoding failure. Flutter's error
  // builder asks the account runtime to refresh the sticker and the next
  // mount must be allowed to load its replacement URL.
  void request.catch(() => {
    if (lottieDataCache.get(source) === request) lottieDataCache.delete(source);
  });
  return request;
}

export function clearLottieStickerCache() {
  lottieDataCache.clear();
}

interface LottieStickerProps {
  source: string;
  alt: string;
  className?: string;
  width?: number;
  height?: number;
  onError?: () => void;
}

export function LottieSticker({ source, alt, className, width, height, onError }: LottieStickerProps) {
  const containerRef = useRef<HTMLSpanElement>(null);
  const animationRef = useRef<AnimationItem | undefined>(undefined);
  const onErrorRef = useRef(onError);
  const [failed, setFailed] = useState(false);
  onErrorRef.current = onError;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    let disposed = false;
    let reportedFailure = false;
    let visible = document.visibilityState === 'visible';
    let animation: AnimationItem | undefined;
    const syncPlayback = () => {
      if (!animation) return;
      if (visible) animation.play();
      else animation.pause();
    };
    const onVisibilityChange = () => {
      visible = document.visibilityState === 'visible';
      syncPlayback();
    };
    document.addEventListener('visibilitychange', onVisibilityChange);
    setFailed(false);

    void Promise.all([loadLottieData(source), loadLottiePlayer()]).then(([animationData, player]) => {
      if (disposed) return;
      animation = player.loadAnimation({
        container,
        renderer: 'svg',
        loop: true,
        autoplay: false,
        animationData,
        rendererSettings: { preserveAspectRatio: 'xMidYMid meet' },
      });
      animationRef.current = animation;
      const fail = () => {
        if (disposed || reportedFailure) return;
        reportedFailure = true;
        lottieDataCache.delete(source);
        void evictCachedResource(source, 'lottie');
        setFailed(true);
        onErrorRef.current?.();
      };
      animation.addEventListener('data_failed', fail);
      animation.addEventListener('error', fail);
      syncPlayback();
    }).catch((reason) => {
      if (disposed || reportedFailure) return;
      reportedFailure = true;
      lottieDataCache.delete(source);
      e('Unable to render Lottie sticker', reason);
      void evictCachedResource(source, 'lottie');
      setFailed(true);
      onErrorRef.current?.();
    });

    return () => {
      disposed = true;
      document.removeEventListener('visibilitychange', onVisibilityChange);
      animation?.destroy();
      if (animationRef.current === animation) animationRef.current = undefined;
    };
  }, [source]);

  if (failed) return <span className={classes("sticker-asset-error")} aria-label={`${alt} unavailable`} style={width || height ? { width, height } : undefined} />;
  return <span ref={containerRef} className={classes(`lottie-sticker${className ? ` ${className}` : ''}`)} role="img" aria-label={alt} style={width || height ? { width, height } : undefined} />;
}
