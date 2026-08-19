import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { resolveAssetUrl } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { evictCachedResource, loadCachedResource } from '@/shared/lib/networkResourceCache';
import { classes } from '@/shared/styles/classes';

export type ImageFit = 'cover' | 'contain' | 'fill' | 'none' | 'scale-down';

interface MixinImageProps {
  source?: string | null;
  alt: string;
  blurHashOrBase64?: string | null;
  revision?: number;
  width?: number | string;
  height?: number | string;
  fit?: ImageFit;
  normalizeGif?: boolean;
  className?: string;
  placeholder?: ReactNode;
  onError?: (error: unknown) => void;
}

const base83 = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#$%*+,-.:;=?@[]^_{|}~';

export interface BlurHashImageData {
  data: Uint8ClampedArray;
  width: number;
  height: number;
}

const maxDownloadedImageBytes = 32 * 1024 * 1024;
const minGifFrameDelayCentiseconds = 2;
const defaultGifFrameDelayCentiseconds = 10;

function decode83(value: string) {
  return [...value].reduce((result, character) => result * 83 + base83.indexOf(character), 0);
}

function srgbToLinear(value: number) {
  const unit = value / 255;
  return unit <= 0.04045 ? unit / 12.92 : Math.pow((unit + 0.055) / 1.055, 2.4);
}

function linearToSrgb(value: number) {
  const unit = Math.max(0, Math.min(1, value));
  return Math.round(255 * (unit <= 0.0031308 ? unit * 12.92 : 1.055 * Math.pow(unit, 1 / 2.4) - 0.055));
}

function signPow(value: number, exponent: number) {
  return Math.sign(value) * Math.pow(Math.abs(value), exponent);
}

/** Matches Flutter's GIF signature check before touching frame delays. */
export function isGifBytes(bytes: Uint8Array) {
  return bytes.length >= 6
    && bytes[0] === 0x47
    && bytes[1] === 0x49
    && bytes[2] === 0x46
    && bytes[3] === 0x38
    && (bytes[4] === 0x37 || bytes[4] === 0x39)
    && bytes[5] === 0x61;
}

/**
 * WebKit can treat a GIF with a zero or one-centisecond frame delay as a
 * corrupt animation. Flutter rewrites those delays while decoding; preserve
 * the same bytes-level rule before passing an image to the WebView decoder.
 */
export function normalizeGifFrameDurations(bytes: Uint8Array) {
  if (!isGifBytes(bytes)) return bytes;
  let normalized: Uint8Array | undefined;
  for (let index = 0; index <= bytes.length - 8; index += 1) {
    if (bytes[index] !== 0x21
      || bytes[index + 1] !== 0xf9
      || bytes[index + 2] !== 0x04
      || bytes[index + 7] !== 0x00) continue;
    const delay = bytes[index + 4] | (bytes[index + 5] << 8);
    if (delay >= minGifFrameDelayCentiseconds) continue;
    normalized ??= new Uint8Array(bytes);
    normalized[index + 4] = defaultGifFrameDelayCentiseconds;
    normalized[index + 5] = 0;
  }
  return normalized ?? bytes;
}

async function normalizedGifBlobUrl(source: string, signal: AbortSignal) {
  // A data/blob URL is already local to the renderer. Blob URLs are immutable,
  // while data URLs are normally tiny thumbnails rather than animated media.
  if (!/^(https?:|asset:|blob:)/.test(source)) return undefined;
  const resource = /^https?:/.test(source)
    ? await loadCachedResource(source, 'image')
    : await (async () => {
      const response = await fetch(source, { signal, cache: 'force-cache' });
      if (!response.ok) throw new Error(`image request failed: ${response.status}`);
      return {
        bytes: new Uint8Array(await response.arrayBuffer()),
        contentType: response.headers.get('content-type') || undefined,
      };
    })();
  if (signal.aborted) return undefined;
  const contentLength = resource.bytes.byteLength;
  if (Number.isFinite(contentLength) && contentLength > maxDownloadedImageBytes) return undefined;
  const bytes = resource.bytes;
  if (bytes.byteLength > maxDownloadedImageBytes) return undefined;
  const normalized = normalizeGifFrameDurations(bytes);
  if (normalized === bytes) return undefined;
  // Copy into an ArrayBuffer: TypeScript permits Uint8Array views backed by a
  // SharedArrayBuffer, which Blob intentionally rejects.
  return URL.createObjectURL(new Blob([new Uint8Array(normalized).buffer], { type: 'image/gif' }));
}

async function cachedImageObjectUrl(source: string) {
  const resource = await loadCachedResource(source, 'image');
  const type = resource.contentType?.split(';', 1)[0] || 'application/octet-stream';
  return URL.createObjectURL(new Blob([new Uint8Array(resource.bytes).buffer], { type }));
}

/** Decodes the same 20x20 BlurHash preview used by Flutter's ImageByBlurHash. */
export function decodeBlurHash(hash: string, width = 20, height = 20): BlurHashImageData | undefined {
  if (hash.length < 6 || [...hash].some((character) => base83.indexOf(character) < 0)) return undefined;
  const sizeFlag = decode83(hash[0]);
  const componentX = sizeFlag % 9 + 1;
  const componentY = Math.floor(sizeFlag / 9) + 1;
  if (hash.length !== 4 + 2 * componentX * componentY) return undefined;
  const maximumValue = (decode83(hash[1]) + 1) / 166;
  const colors: Array<[number, number, number]> = [];
  for (let index = 0; index < componentX * componentY; index += 1) {
    const value = decode83(hash.slice(4 + index * 2, 6 + index * 2));
    if (index === 0) {
      colors.push([
        srgbToLinear(value >> 16),
        srgbToLinear((value >> 8) & 255),
        srgbToLinear(value & 255),
      ]);
    } else {
      const quantR = Math.floor(value / (19 * 19));
      const quantG = Math.floor(value / 19) % 19;
      const quantB = value % 19;
      colors.push([
        signPow((quantR - 9) / 9, 2) * maximumValue,
        signPow((quantG - 9) / 9, 2) * maximumValue,
        signPow((quantB - 9) / 9, 2) * maximumValue,
      ]);
    }
  }
  const data = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let red = 0;
      let green = 0;
      let blue = 0;
      for (let j = 0; j < componentY; j += 1) {
        for (let i = 0; i < componentX; i += 1) {
          const basis = Math.cos(Math.PI * x * i / width) * Math.cos(Math.PI * y * j / height);
          const color = colors[i + j * componentX];
          red += color[0] * basis;
          green += color[1] * basis;
          blue += color[2] * basis;
        }
      }
      const offset = 4 * (x + y * width);
      data[offset] = linearToSrgb(red);
      data[offset + 1] = linearToSrgb(green);
      data[offset + 2] = linearToSrgb(blue);
      data[offset + 3] = 255;
    }
  }
  return { data, width, height };
}

function placeholderUrl(value?: string | null) {
  if (!value) return undefined;
  try {
    const image = decodeBlurHash(value);
    if (image) {
      const canvas = document.createElement('canvas');
      canvas.width = image.width;
      canvas.height = image.height;
      canvas.getContext('2d')?.putImageData(new ImageData(new Uint8ClampedArray(image.data), image.width, image.height), 0, 0);
      return canvas.toDataURL('image/png');
    }
  } catch { /* fall through to the base64 form below */ }
  try {
    atob(value.replace(/^data:[^,]+,/, ''));
    return value.startsWith('data:') ? value : `data:image/png;base64,${value}`;
  } catch {
    return undefined;
  }
}

function withRevision(source: string, revision: number) {
  if (!revision || source.startsWith('data:') || source.startsWith('blob:')) return source;
  const separator = source.includes('?') ? '&' : '?';
  return `${source}${separator}mixin_image_revision=${encodeURIComponent(revision)}`;
}

/**
 * Source-parity replacement for Flutter MixinImage/MediaImagePipeline.
 * Browser and WebView image caching is deliberately used for the network cache;
 * revision creates a distinct cache key just like ProxyNetworkImage.
 */
export function MixinImage({
  source,
  alt,
  blurHashOrBase64,
  revision = 0,
  width,
  height,
  fit = 'cover',
  normalizeGif = false,
  className = '',
  placeholder,
  onError,
}: MixinImageProps) {
  const resolved = useMemo(() => {
    const value = resolveAssetUrl(source);
    return value ? withRevision(value, revision) : undefined;
  }, [revision, source]);
  const fallback = useMemo(() => placeholderUrl(blurHashOrBase64), [blurHashOrBase64]);
  const [networkSource, setNetworkSource] = useState<string>();
  const cacheRetryKey = useRef<string | undefined>(undefined);
  const [cacheRetry, setCacheRetry] = useState(0);
  const isNetworkSource = Boolean(resolved && /^https?:/.test(resolved));
  useEffect(() => {
    cacheRetryKey.current = undefined;
  }, [resolved]);
  useEffect(() => {
    if (!resolved || !isNetworkSource) {
      setNetworkSource(resolved);
      return undefined;
    }
    let disposed = false;
    let objectUrl: string | undefined;
    setNetworkSource(undefined);
    void cachedImageObjectUrl(resolved).then((value) => {
      if (disposed) {
        URL.revokeObjectURL(value);
        return;
      }
      objectUrl = value;
      setNetworkSource(value);
    }).catch((reason) => {
      if (disposed) return;
      e('Unable to cache network image', reason);
      setNetworkSource(resolved);
    });
    return () => {
      disposed = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [isNetworkSource, resolved, cacheRetry]);
  const [normalizedSource, setNormalizedSource] = useState<string | undefined>();
  useEffect(() => {
    const sourceForNormalization = isNetworkSource ? networkSource : resolved;
    if (!sourceForNormalization || !normalizeGif) {
      setNormalizedSource(undefined);
      return undefined;
    }
    const controller = new AbortController();
    let objectUrl: string | undefined;
    setNormalizedSource(undefined);
    void normalizedGifBlobUrl(sourceForNormalization, controller.signal).then((value) => {
      if (controller.signal.aborted || !value) return;
      objectUrl = value;
      setNormalizedSource(value);
    }).catch((reason) => {
      e('Unable to normalize GIF image', reason);
      // The native image element remains the error boundary for inaccessible
      // URLs, matching Flutter's errorBuilder rather than hiding the failure.
    });
    return () => {
      controller.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [isNetworkSource, networkSource, normalizeGif, resolved]);
  const displaySource = normalizedSource ?? networkSource;
  const [state, setState] = useState<'loading' | 'loaded' | 'error'>('loading');
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    setState('loading');
    return () => { mounted.current = false; };
  }, [displaySource]);
  const style = { width, height, objectFit: fit } as CSSProperties;
  return (
    <span className={classes(`mixin-image ${className}`.trim())} style={{ width, height }}>
      {state !== 'loaded' && (placeholder ?? (fallback && <img className={classes("mixin-image-placeholder")} src={fallback} alt="" aria-hidden="true" style={style} />))}
      {displaySource && <img className={classes(state === 'loaded' ? 'is-loaded' : '')} src={displaySource} alt={alt} draggable={false} style={style} onLoad={() => { if (mounted.current) setState('loaded'); }} onError={(error) => {
        if (isNetworkSource && resolved && cacheRetryKey.current !== resolved) {
          cacheRetryKey.current = resolved;
          void evictCachedResource(resolved, 'image').then(() => {
            if (mounted.current) setCacheRetry((value) => value + 1);
          });
          return;
        }
        if (mounted.current) setState('error');
        onError?.(error);
      }} />}
    </span>
  );
}
