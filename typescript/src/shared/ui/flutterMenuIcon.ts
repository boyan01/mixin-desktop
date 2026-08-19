import iconFontUrl from '@flutter-assets/fonts/iconfont.ttf';

export type FlutterMenuIconName =
  | 'chat'
  | 'circle'
  | 'copy'
  | 'delete'
  | 'download'
  | 'edit'
  | 'forward'
  | 'manageCircle'
  | 'manageUser'
  | 'mute'
  | 'pin'
  | 'positionToChat'
  | 'recall'
  | 'reply'
  | 'select'
  | 'sticker'
  | 'unMute'
  | 'unPin';

const codePoints: Record<FlutterMenuIconName, number> = {
  chat: 0xe7a4,
  circle: 0xe7a8,
  copy: 0xe798,
  delete: 0xe797,
  download: 0xe7a1,
  edit: 0xe7a7,
  forward: 0xe795,
  manageCircle: 0xe7a2,
  manageUser: 0xe7a3,
  mute: 0xe601,
  pin: 0xe796,
  positionToChat: 0xe7a6,
  recall: 0xe799,
  reply: 0xe79a,
  select: 0xe7af,
  sticker: 0xe7a0,
  unMute: 0xe79b,
  unPin: 0xe79c,
};

const fontReady = loadIconFont();
const iconCache = new Map<string, Promise<Uint8Array>>();

async function loadIconFont() {
  const face = await new FontFace('mixin-menu-icons', `url(${iconFontUrl})`).load();
  document.fonts.add(face);
}

function renderIcon(name: FlutterMenuIconName, color: string) {
  const cacheKey = `${name}:${color}`;
  const cached = iconCache.get(cacheKey);
  if (cached) return cached;
  const icon = fontReady.then(async () => {
    const canvas = document.createElement('canvas');
    canvas.width = 18;
    canvas.height = 18;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Canvas 2D context is unavailable');
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = color;
    context.font = '16px mixin-menu-icons';
    context.textAlign = 'center';
    context.textBaseline = 'middle';
    context.fillText(String.fromCodePoint(codePoints[name]), canvas.width / 2, canvas.height / 2);
    const blob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((value) => value ? resolve(value) : reject(new Error('Unable to encode native menu icon')), 'image/png');
    });
    return new Uint8Array(await blob.arrayBuffer());
  });
  iconCache.set(cacheKey, icon);
  return icon;
}

function nativeMenuIconColor() {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? '#F0F0F0' : '#090909';
}

export function flutterMenuIcon(name: FlutterMenuIconName) {
  return renderIcon(name, nativeMenuIconColor());
}
