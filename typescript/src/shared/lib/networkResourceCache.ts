import { e } from '@/shared/lib/logger';

export type NetworkResourceKind = 'image' | 'lottie';

export interface CachedResource {
  bytes: Uint8Array;
  contentType?: string;
}

const cacheNames: Record<NetworkResourceKind, string> = {
  image: 'mixin-image-v1',
  lottie: 'mixin-lottie-v1',
};
const pendingResources = new Map<string, Promise<CachedResource>>();

function pendingKey(kind: NetworkResourceKind, source: string) {
  return `${kind}:${source}`;
}

async function readResponse(response: Response): Promise<CachedResource> {
  return {
    bytes: new Uint8Array(await response.arrayBuffer()),
    contentType: response.headers.get('content-type') || undefined,
  };
}

async function openCache(kind: NetworkResourceKind) {
  if (typeof caches === 'undefined') return undefined;
  try {
    return await caches.open(cacheNames[kind]);
  } catch (reason) {
    e(`Unable to open ${kind} resource cache`, reason);
    return undefined;
  }
}

export async function loadCachedResource(source: string, kind: NetworkResourceKind, refresh = false): Promise<CachedResource> {
  const cache = await openCache(kind);
  if (refresh) {
    try {
      await cache?.delete(source);
    } catch (reason) {
      e(`Unable to evict ${kind} resource cache`, reason);
    }
  }

  const key = pendingKey(kind, source);
  if (!refresh) {
    const pending = pendingResources.get(key);
    if (pending) return pending;
  }

  const request = (async () => {
    const cached = await cache?.match(source);
    if (cached) return readResponse(cached);

    const response = await fetch(source, { cache: cache ? 'no-store' : 'force-cache' });
    if (!response.ok) throw new Error(`${kind} request failed: ${response.status}`);
    try {
      await cache?.put(source, response.clone());
    } catch (reason) {
      e(`Unable to persist ${kind} resource cache`, reason);
    }
    return readResponse(response);
  })();
  pendingResources.set(key, request);
  void request.then(
    () => {
      if (pendingResources.get(key) === request) pendingResources.delete(key);
    },
    () => {
      if (pendingResources.get(key) === request) pendingResources.delete(key);
    },
  );
  return request;
}

export async function evictCachedResource(source: string, kind: NetworkResourceKind) {
  const cache = await openCache(kind);
  try {
    await cache?.delete(source);
  } catch (reason) {
    e(`Unable to evict ${kind} resource cache`, reason);
  }
}
