import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { Sticker } from '@/shared/model/types';
import { LottieSticker } from '@/shared/ui/LottieSticker';
import { MixinImage } from '@/shared/ui/MixinImage';
import { classes } from '@/shared/styles/classes';

export function PickerStickerAsset({ sticker, className }: { sticker: Sticker; className?: string }) {
  const source = resolveAssetUrl(sticker.assetUrl);
  const refresh = () => queueMicrotask(() => { void desktopApi.refreshSticker(sticker.stickerId).catch((reason) => e('Unable to refresh sticker asset', reason)); });
  if (!source) return <span className={classes("sticker-asset-error")} />;
  if (sticker.assetType === 'json') return <LottieSticker source={source} alt={sticker.name} className={classes(className)} onError={refresh} />;
  return <MixinImage className={classes(className)} source={source} alt={sticker.name} fit="contain" normalizeGif={sticker.assetType.toLowerCase() === 'gif'} onError={refresh} />;
}
