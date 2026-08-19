import { useEffect, useState } from 'react';
import { ArrowLeft, Settings, Trash2, X } from 'lucide-react';
import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { Sticker, StickerAlbum, StickerDetail } from '@/shared/model/types';
import { showToastFailed } from '@/shared/ui/toast';
import { LottieSticker } from '@/shared/ui/LottieSticker';
import { MixinImage } from '@/shared/ui/MixinImage';
import { classes } from '@/shared/styles/classes';

function StickerAsset({ sticker, size }: { sticker: Sticker; size?: number }) {
  const [failed, setFailed] = useState(false);
  const source = resolveAssetUrl(sticker.assetUrl);
  if (!source || failed) return <span className={classes("sticker-asset-error")} style={size ? { width: size, height: size } : undefined} />;
  const refresh = () => {
    setFailed(true);
    queueMicrotask(() => { void desktopApi.refreshSticker(sticker.stickerId).catch((reason) => e('Unable to refresh sticker asset', reason)); });
  };
  if (sticker.assetType === 'json') return <LottieSticker source={source} alt={sticker.name} onError={refresh} width={size} height={size} />;
  return <MixinImage source={source} alt={sticker.name} width={size} height={size} fit="contain" normalizeGif={sticker.assetType.toLowerCase() === 'gif'} onError={refresh} />;
}

export function StickerDetailDialog({ stickerId, onClose, onAlbumChanged }: { stickerId: string; onClose: () => void; onAlbumChanged?: () => Promise<void> | void }) {
  const [detail, setDetail] = useState<StickerDetail>();
  const [selected, setSelected] = useState<Sticker>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void desktopApi.stickerDetail(stickerId).then((value) => {
      if (!active) return;
      setDetail(value);
      setSelected(value.sticker);
    }).catch(showToastFailed);
    return () => { active = false; };
  }, [stickerId]);

  async function toggleAlbum() {
    if (!detail?.album || saving) return;
    setSaving(true);
    const added = !detail.album.added;
    try {
      await desktopApi.setStickerAlbumAdded(detail.album.albumId, added);
      setDetail((current) => current ? { ...current, album: current.album ? { ...current.album, added } : current.album } : current);
      await onAlbumChanged?.();
    } catch (error) {
      showToastFailed(error);
    } finally {
      setSaving(false);
    }
  }

  return <div className={classes("modal-backdrop sticker-detail-backdrop")} onMouseDown={onClose}>
    <section className={classes("sticker-detail-dialog")} role="dialog" aria-modal="true" aria-label="Sticker detail" onMouseDown={(event) => event.stopPropagation()}>
      <header><span /><button type="button" aria-label="Close sticker detail" onClick={onClose}><X size={20} /></button></header>
      <div className={classes("sticker-detail-preview")}>{selected && <StickerAsset sticker={selected} size={256} />}</div>
      {detail?.album && <div className={classes("sticker-detail-album")}><strong>{detail.album.name}</strong><button type="button" disabled={saving} className={classes(detail.album.added ? 'is-remove' : '')} onClick={() => void toggleAlbum()}>{detail.album.added ? 'Remove Stickers' : 'Add Stickers'}</button></div>}
      {detail?.album && detail.albumStickers.length > 0 && <nav className={classes("sticker-detail-tabs")}>{detail.albumStickers.map((sticker) => <button type="button" key={sticker.stickerId} className={classes(selected?.stickerId === sticker.stickerId ? 'is-selected' : '')} onClick={() => setSelected(sticker)}><StickerAsset sticker={sticker} size={64} /></button>)}</nav>}
    </section>
  </div>;
}

type StorePage = { kind: 'store' } | { kind: 'album'; album: StickerAlbum } | { kind: 'manage' };

export function StickerStoreDialog({ albums, stickers, onClose, onToggle, onAlbumsChanged }: { albums: StickerAlbum[]; stickers: Record<string, Sticker[]>; onClose: () => void; onToggle: (album: StickerAlbum) => Promise<void>; onAlbumsChanged?: () => Promise<void> | void }) {
  const [page, setPage] = useState<StorePage>({ kind: 'store' });
  const [orderedAlbums, setOrderedAlbums] = useState(albums);
  const [draggingId, setDraggingId] = useState<string>();
  useEffect(() => setOrderedAlbums(albums), [albums]);

  async function removeAlbum(album: StickerAlbum) {
    try {
      await onToggle(album);
      setOrderedAlbums((current) => current.filter((item) => item.albumId !== album.albumId));
      await onAlbumsChanged?.();
    } catch { /* the parent presents the command error */ }
  }

  async function dropAlbum(targetId: string) {
    if (!draggingId || draggingId === targetId) return;
    const next = [...orderedAlbums];
    const from = next.findIndex((item) => item.albumId === draggingId);
    const to = next.findIndex((item) => item.albumId === targetId);
    if (from < 0 || to < 0) return;
    const [album] = next.splice(from, 1);
    next.splice(to, 0, album);
    setOrderedAlbums(next);
    try {
      await desktopApi.setStickerAlbumOrder(next.map((item) => item.albumId));
      await onAlbumsChanged?.();
    } catch (error) { showToastFailed(error); }
    finally { setDraggingId(undefined); }
  }

  const header = page.kind === 'store' ? <><button type="button" aria-label="Manage stickers" onClick={() => setPage({ kind: 'manage' })}><Settings size={20} /></button><strong>Sticker Store</strong></> : <><button type="button" aria-label="Back" onClick={() => setPage({ kind: 'store' })}><ArrowLeft size={20} /></button><strong>{page.kind === 'manage' ? 'My Stickers' : page.album.name}</strong></>;
  return <div className={classes("modal-backdrop sticker-store-backdrop")} onMouseDown={onClose}><section className={classes("sticker-store-dialog")} role="dialog" aria-modal="true" onMouseDown={(event) => event.stopPropagation()}><header>{header}<button type="button" aria-label="Close sticker store" onClick={onClose}><X size={20} /></button></header><main>
    {page.kind === 'store' && <div className={classes("sticker-store-page")}>{albums.map((album) => <section key={album.albumId}><h3>{album.name}</h3><div>{(stickers[album.albumId] ?? []).slice(0, 4).map((sticker) => <button type="button" className={classes("sticker-store-preview")} key={sticker.stickerId} onClick={() => setPage({ kind: 'album', album })}><StickerAsset sticker={sticker} size={72} /></button>)}<button type="button" className={classes(album.added ? 'is-added' : '')} onClick={() => void onToggle(album)}>{album.added ? 'Added' : 'Add'}</button></div></section>)}</div>}
    {page.kind === 'album' && <StoreAlbumPage album={page.album} stickers={stickers[page.album.albumId] ?? []} onToggle={onToggle} />}
    {page.kind === 'manage' && <div className={classes("sticker-album-manage")}>{orderedAlbums.map((album) => <div key={album.albumId} draggable onDragStart={() => setDraggingId(album.albumId)} onDragOver={(event) => event.preventDefault()} onDrop={() => void dropAlbum(album.albumId)}><StickerAsset sticker={{ stickerId: album.albumId, albumId: null, name: album.name, assetUrl: album.iconUrl, assetWidth: 72, assetHeight: 72, assetType: album.iconUrl.endsWith('.json') ? 'json' : '' }} size={72} /><strong>{album.name}</strong><button type="button" aria-label={`Remove ${album.name}`} onClick={() => void removeAlbum(album)}><Trash2 size={20} /></button></div>)}</div>}
  </main></section></div>;
}

function StoreAlbumPage({ album, stickers, onToggle }: { album: StickerAlbum; stickers: Sticker[]; onToggle: (album: StickerAlbum) => Promise<void> }) {
  const [added, setAdded] = useState(album.added);
  const [detailId, setDetailId] = useState<string>();
  async function toggle() {
    await onToggle({ ...album, added });
    setAdded((value) => !value);
  }
  return <><div className={classes("sticker-store-album-grid")}>{stickers.map((sticker) => <button type="button" key={sticker.stickerId} onClick={() => setDetailId(sticker.stickerId)}><StickerAsset sticker={sticker} /></button>)}</div><footer className={classes("sticker-store-album-footer")}><button type="button" className={classes(added ? 'is-remove' : '')} onClick={() => void toggle()}>{added ? 'Remove Stickers' : 'Add Stickers'}</button></footer>{detailId && <StickerDetailDialog stickerId={detailId} onClose={() => setDetailId(undefined)} />}</>;
}
