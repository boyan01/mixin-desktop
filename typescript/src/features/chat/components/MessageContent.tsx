import {
  useCallback,
  useEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { save } from '@tauri-apps/plugin-dialog';
import {
  AlertTriangle,
  Copy,
  Download,
  FileText,
  Forward,
  Pause,
  Play,
  RotateCw,
  Undo2,
  Volume2,
  VolumeX,
  X,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import { e } from '@/shared/lib/logger';
import { useAudioPlayback } from '@/shared/media/audioPlayback';
import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { openBotWebview } from '@/shared/platform/botWebview';
import { openExternal } from '@/shared/lib/openExternal';
import { useDesktopUi } from '@/state/desktopUi';
import type { Message } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { IconButton } from '@/shared/ui/IconButton';
import { LottieSticker } from '@/shared/ui/LottieSticker';
import { MarkdownColumn, SelectableMessageText } from '@/shared/ui/RichText';
import { MixinImage } from '@/shared/ui/MixinImage';
import { showToastFailed, showToastSuccessful } from '@/shared/ui/toast';
import { StickerDetailDialog } from '@/features/chat/components/StickerDetailDialog';
import { SnapshotDetailDialog as SnapshotDetailPage } from '@/features/wallet/components/SnapshotDetailDialog';
import { inscriptionColors, inscriptionDisplayContent } from '@/features/chat/lib/inscription';
import {
  actionButtonRows,
  canForwardMessage,
  formatDuration,
  hasInvalidSpecialPayload,
  isCategory,
  mapTilePosition,
  messageNameColor,
} from '@/features/chat/model/message';
import locationMarkUrl from '@/features/chat/assets/location_mark.svg';
import externalLinkUrl from '@flutter-assets/images/external_link.svg';
import linkSendUrl from '@flutter-assets/images/link_send.svg';
import inscriptionPlaceholderUrl from '@flutter-assets/images/inscription_placeholder.svg';
import collectionPlaceholderUrl from '@flutter-assets/images/collection_placeholder.svg';
import inscriptionTextBackgroundUrl from '@flutter-assets/images/text_bg.png';
import { classes } from '@/shared/styles/classes';

interface QuoteData {
  senderId?: string;
  sender?: string;
  category: string;
  content: string;
  mediaName?: string;
  mediaUrl?: string;
  thumbUrl?: string;
  thumbImage?: string;
  stickerAssetUrl?: string;
  stickerAssetType?: string;
  sharedUserName?: string;
  sharedUserId?: string;
  sharedUserAvatarUrl?: string;
  sharedUserIdentityNumber?: string;
}

function quoteData(raw?: string | null): QuoteData | undefined {
  if (!raw) return undefined;
  try {
    const value = JSON.parse(raw) as Record<string, unknown>;
    return {
      senderId: typeof value.user_id === 'string' ? value.user_id : undefined,
      sender: typeof value.user_full_name === 'string' ? value.user_full_name : undefined,
      category: typeof value.type === 'string' ? value.type : '',
      content: typeof value.content === 'string' ? value.content : '',
      mediaName: typeof value.media_name === 'string' ? value.media_name : undefined,
      mediaUrl: typeof value.media_url === 'string' ? value.media_url : undefined,
      thumbUrl: typeof value.thumb_url === 'string' ? value.thumb_url : undefined,
      thumbImage: typeof value.thumb_image === 'string' ? value.thumb_image : undefined,
      stickerAssetUrl: typeof value.asset_url === 'string' ? value.asset_url : undefined,
      stickerAssetType: typeof value.asset_type === 'string' ? value.asset_type : undefined,
      sharedUserName: typeof value.shared_user_full_name === 'string' ? value.shared_user_full_name : undefined,
      sharedUserId: typeof value.shared_user_id === 'string' ? value.shared_user_id : undefined,
      sharedUserAvatarUrl: typeof value.shared_user_avatar_url === 'string' ? value.shared_user_avatar_url : undefined,
      sharedUserIdentityNumber: typeof value.shared_user_identity_number === 'string' ? value.shared_user_identity_number : undefined,
    };
  } catch { return undefined; }
}

function quoteIsSupported(category: string) {
  return ['TEXT', 'IMAGE', 'VIDEO', 'LIVE', 'AUDIO', 'DATA', 'STICKER', 'CONTACT', 'LOCATION', 'TRANSCRIPT', 'POST']
    .some((suffix) => isCategory(category, suffix))
    || category === 'APP_CARD'
    || category === 'APP_BUTTON_GROUP';
}

function quotePreviewText(quote: QuoteData, mentionNames: Record<string, string>) {
  if (isCategory(quote.category, 'TEXT')) return quote.content.replace(/@(\d{4,})/g, (_, id: string) => `@${mentionNames[id] || id}`);
  if (isCategory(quote.category, 'POST')) return quote.content.replace(/[#*_`]/g, '').split('\n')[0] || '';
  if (isCategory(quote.category, 'IMAGE')) return 'Image';
  if (isCategory(quote.category, 'VIDEO')) return 'Video';
  if (isCategory(quote.category, 'LIVE')) return 'Live';
  if (isCategory(quote.category, 'AUDIO')) return 'Audio';
  if (isCategory(quote.category, 'DATA')) return quote.mediaName || 'File';
  if (isCategory(quote.category, 'STICKER')) return 'Sticker';
  if (isCategory(quote.category, 'CONTACT')) return quote.sharedUserIdentityNumber || quote.sharedUserName || 'Contact';
  if (isCategory(quote.category, 'LOCATION')) return 'Location';
  if (isCategory(quote.category, 'TRANSCRIPT')) return 'Transcript';
  if (quote.category === 'APP_CARD') { try { return String((JSON.parse(quote.content) as { title?: unknown }).title || 'Bots'); } catch { return 'Bots'; } }
  if (quote.category === 'APP_BUTTON_GROUP') { try { return (JSON.parse(quote.content) as Array<{ label?: string }>).map((item) => `[${item.label || ''}]`).join(''); } catch { return 'Bots'; } }
  return 'Message not supported';
}

function QuotePreview({ raw, membership, mentionNames, onLocate }: { raw?: string | null; membership?: string | null; mentionNames: Record<string, string>; onLocate?: () => void }) {
  const quote = quoteData(raw);
  if (!quote) return <button type="button" className={classes("message-quote")} disabled={!onLocate} onClick={onLocate}><span className={classes("quote-accent")} /><span><small>Message not found</small></span></button>;
  if (!quoteIsSupported(quote.category)) return <button type="button" className={classes("message-quote")} disabled={!onLocate} onClick={onLocate}><span className={classes("quote-accent")} /><span><small>Message not supported</small></span></button>;
  const source = resolveAssetUrl(quote.stickerAssetUrl || (isCategory(quote.category, 'IMAGE') ? quote.mediaUrl : quote.thumbUrl));
  const image = isCategory(quote.category, 'CONTACT')
    ? <Avatar name={quote.sharedUserName || ''} url={quote.sharedUserAvatarUrl} size={48} />
    : source && <MixinImage source={source} alt="" blurHashOrBase64={quote.thumbImage} />;
  return <button type="button" className={classes("message-quote")} disabled={!onLocate} onClick={onLocate}><span className={classes("quote-accent")} style={{ background: messageNameColor(quote.senderId || '') }} /><span className={classes("quote-copy")}>{quote.sender && <strong style={{ '--sender-color': messageNameColor(quote.senderId || '') } as CSSProperties}>{quote.sender}<Badges verified={false} isBot={false} membership={membership} /></strong>}<small>{quotePreviewText(quote, mentionNames)}</small></span>{image}</button>;
}

function AudioMessage({ message, currentUserId, playlist = [] }: { message: Message; currentUserId?: string; playlist?: Message[] }) {
  const currentMessageId = useAudioPlayback((state) => state.message?.messageId);
  const playing = useAudioPlayback((state) => state.isPlaying);
  const progress = useAudioPlayback((state) => state.message?.messageId === message.messageId ? state.progress : 0);
  const play = useAudioPlayback((state) => state.play);
  const stop = useAudioPlayback((state) => state.stop);
  const source = resolveAssetUrl(message.mediaUrl);
  const status = message.mediaStatus.toUpperCase();
  const mine = message.senderId === currentUserId;
  const waveform = useMemo(() => {
    try {
      const bytes = Uint8Array.from(atob(message.mediaWaveform || ''), (char) => char.charCodeAt(0));
      return bytes.length ? [...bytes].map((value) => Math.max(2, Math.round(value / 255 * 12))) : Array.from({ length: 48 }, (_, index) => 3 + (index * 13 % 9));
    } catch { return Array.from({ length: 48 }, (_, index) => 3 + (index * 13 % 9)); }
  }, [message.mediaWaveform]);

  async function toggle() {
    if (status === 'CANCELED') {
      await desktopApi.downloadAttachment(message.messageId);
      return;
    }
    if (status === 'PENDING') {
      await desktopApi.cancelAttachment(message.messageId);
      return;
    }
    if (!source || !['DONE', 'READ'].includes(status)) return;
    if (currentMessageId === message.messageId && playing) stop();
    else {
      await play(message, source, playlist
        .filter((item) => item.category.endsWith('_AUDIO') && ['DONE', 'READ'].includes(item.mediaStatus.toUpperCase()))
        .map((item) => ({ message: item, source: resolveAssetUrl(item.mediaUrl) || '' }))
        .filter((item) => item.source));
      if (status === 'DONE') void desktopApi.markAudioRead(message.messageId);
    }
  }

  const readWaveform = mine || status === 'READ';
  const canInteract = status !== 'EXPIRED' && (status === 'CANCELED' || status === 'PENDING' || Boolean(source));
  const control = status === 'CANCELED'
    ? mine && message.mediaUrl ? <RotateCw size={17} /> : <Download size={17} />
    : status === 'PENDING' ? <span className={classes("audio-pending")} />
    : status === 'EXPIRED' ? <AlertTriangle size={17} />
    : playing ? <Pause size={17} fill="currentColor" /> : <Play size={17} fill="currentColor" />;
  return (
    <button className={classes(`audio-message ${readWaveform ? 'is-read' : ''}`)} type="button" disabled={!canInteract} onClick={() => void toggle()}>
      <span className={classes("audio-play")}>{control}</span>
      <span className={classes("audio-waveform")} style={{ '--audio-progress': progress } as CSSProperties}>{waveform.map((height, index) => <i key={index} style={{ height }} />)}<small>{formatDuration(message.mediaDuration)}</small></span>
    </button>
  );
}

async function handleAttachment(message: Message, onOpen: () => void) {
  const status = message.mediaStatus.toUpperCase();
  if (status === 'CANCELED') {
    await desktopApi.downloadAttachment(message.messageId);
    return;
  }
  if (status === 'PENDING') {
    await desktopApi.cancelAttachment(message.messageId);
    return;
  }
  if (status === 'DONE') onOpen();
}

function AttachmentOverlay({ message }: { message: Message }) {
  const status = message.mediaStatus.toUpperCase();
  if (status === 'CANCELED') return <span className={classes("media-status-overlay")}><Download size={18} /></span>;
  if (status === 'PENDING') return <span className={classes("media-status-overlay")}><span className={classes("audio-pending")} /></span>;
  if (status === 'EXPIRED') return <span className={classes("media-status-overlay")}><AlertTriangle size={18} /></span>;
  return null;
}

interface ImagePreviewEntry {
  message: Message;
  source: string;
}

export function imagePreviewEntries(messages: Message[]) {
  return messages.flatMap((message) => {
    const source = resolveAssetUrl(message.mediaUrl);
    return isCategory(message.category, 'IMAGE') && source ? [{ message, source }] : [];
  });
}

function ImageMessage({ message, source, mentionNames, onOpenIdentity, imagePlaylist = [] }: { message: Message; source: string; mentionNames: Record<string, string>; onOpenIdentity?: (identityNumber: string) => void; imagePlaylist?: Message[] }) {
  const [open, setOpen] = useState(false);
  const sourceWidth = Math.max(1, message.mediaWidth ?? 1);
  const sourceHeight = Math.max(1, message.mediaHeight ?? 1);
  const mediaStyle = {
    '--media-source-width': `${sourceWidth / window.devicePixelRatio}px`,
    aspectRatio: `${sourceWidth} / ${sourceHeight}`,
  } as CSSProperties;
  const images = useMemo(() => {
    const entries = imagePreviewEntries(imagePlaylist);
    return entries.some((entry) => entry.message.messageId === message.messageId) ? entries : [{ message, source }];
  }, [imagePlaylist, message, source]);
  return <><div className={classes("media-message image-message")}><button type="button" className={classes("media-open")} style={mediaStyle} onClick={() => void handleAttachment(message, () => setOpen(true))}><MixinImage source={source} alt={message.caption || 'Attachment'} blurHashOrBase64={message.thumbImage} /><AttachmentOverlay message={message} /></button>{message.caption && <SelectableMessageText className={classes("media-caption")} content={message.caption} mentionNames={mentionNames} onOpenIdentity={onOpenIdentity} />}</div>{open && <MediaPreview message={message} source={source} name={message.mediaName || message.caption || 'Image'} images={images} onClose={() => setOpen(false)} />}</>;
}

function VideoMessage({ message, source, poster, mentionNames, onOpenIdentity }: { message: Message; source: string; poster?: string; mentionNames: Record<string, string>; onOpenIdentity?: (identityNumber: string) => void }) {
  const [open, setOpen] = useState(false);
  const width = Math.min(message.mediaWidth ?? 200, 200);
  const aspectRatio = (message.mediaWidth ?? 200) / (message.mediaHeight ?? 200);
  const mediaSize = { width, height: width / aspectRatio };
  return <><div className={classes("media-message video-message")} style={{ width }}><button type="button" className={classes("media-open")} style={mediaSize} onClick={() => void handleAttachment(message, () => setOpen(true))}><MixinImage source={poster || source} alt="" blurHashOrBase64={message.thumbImage} /><AttachmentOverlay message={message} />{message.mediaStatus.toUpperCase() === 'DONE' && <span className={classes("video-play")}><Play size={24} fill="currentColor" /></span>}</button><small className={classes("media-duration")}>{formatDuration(message.mediaDuration)}</small>{message.caption && <SelectableMessageText className={classes("media-caption")} content={message.caption} mentionNames={mentionNames} onOpenIdentity={onOpenIdentity} />}</div>{open && <MediaPreview message={message} source={source} name={message.mediaName || 'Video'} video onClose={() => setOpen(false)} />}</>;
}

function MediaPreview({ message, source, name, video = false, onClose, images = [] }: { message: Message; source: string; name: string; video?: boolean; onClose: () => void; images?: ImagePreviewEntry[] }) {
  const [zoom, setZoom] = useState(1);
  const [rotation, setRotation] = useState(0);
  const [translation, setTranslation] = useState({ x: 0, y: 0 });
  const [index, setIndex] = useState(() => Math.max(0, images.findIndex((entry) => entry.message.messageId === message.messageId)));
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef<{ x: number; y: number; zoom: number; distance?: number } | undefined>(undefined);
  const openForward = useDesktopUi((state) => state.openForward);
  const selected = !video && images.length ? images[Math.min(index, images.length - 1)] : { message, source };
  const selectedMessage = selected.message;
  const selectedSource = selected.source;
  const selectedName = selectedMessage.mediaName || selectedMessage.caption || name;
  const select = useCallback((next: number) => {
    if (next < 0 || next >= images.length) return;
    setIndex(next);
    setZoom(1);
    setRotation(0);
    setTranslation({ x: 0, y: 0 });
  }, [images.length]);
  useEffect(() => {
    function onKeyDown(event: globalThis.KeyboardEvent) {
      if (event.key === 'Escape') onClose();
      if (!video && event.key === 'ArrowLeft') select(index - 1);
      if (!video && event.key === 'ArrowRight') select(index + 1);
      if (!video && (event.key === '+' || event.key === '=')) setZoom((value) => Math.min(5, value * 1.25));
      if (!video && event.key === '-') setZoom((value) => Math.max(.5, value * .8));
      if (!video && event.key.toLowerCase() === 'r') setRotation((value) => (value + 90) % 360);
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [index, onClose, select, video]);
  async function download() {
    const destination = await save({ defaultPath: name });
    if (destination) await desktopApi.saveAttachmentAs(selectedMessage.messageId, destination);
  }
  async function copy() {
    try {
      const response = await fetch(selectedSource);
      const blob = await response.blob();
      await navigator.clipboard.write([new ClipboardItem({ [blob.type || 'image/png']: blob })]);
      showToastSuccessful();
    } catch (error) { showToastFailed(error); }
  }
  const pointerDown = (event: ReactPointerEvent<HTMLImageElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    pointers.current.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const points = [...pointers.current.values()];
    gesture.current = { x: event.clientX, y: event.clientY, zoom, distance: points.length > 1 ? Math.hypot(points[0].x - points[1].x, points[0].y - points[1].y) : undefined };
  };
  const pointerMove = (event: ReactPointerEvent<HTMLImageElement>) => {
    if (!pointers.current.has(event.pointerId) || !gesture.current) return;
    pointers.current.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const points = [...pointers.current.values()];
    if (points.length > 1 && gesture.current.distance) {
      const distance = Math.hypot(points[0].x - points[1].x, points[0].y - points[1].y);
      setZoom(Math.max(.5, Math.min(5, gesture.current.zoom * distance / gesture.current.distance)));
    } else {
      setTranslation((value) => ({ x: value.x + event.clientX - gesture.current!.x, y: value.y + event.clientY - gesture.current!.y }));
      gesture.current.x = event.clientX;
      gesture.current.y = event.clientY;
    }
  };
  const pointerUp = (event: ReactPointerEvent<HTMLImageElement>) => {
    pointers.current.delete(event.pointerId);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (pointers.current.size === 0) gesture.current = undefined;
  };
  return <div className={classes("media-preview")} role="dialog" aria-modal="true" aria-label={selectedName} onClick={onClose}><header onClick={(event) => event.stopPropagation()}><Avatar name={selectedMessage.senderName} url={selectedMessage.senderAvatarUrl} size={36} /><span><strong>{selectedMessage.senderName}</strong><small>{selectedMessage.senderIdentityNumber || ''}</small></span><nav>{canForwardMessage(selectedMessage) && <IconButton label="Forward" onClick={() => openForward([selectedMessage.messageId])}><Forward size={20} /></IconButton>}{!video && <IconButton label="Copy" onClick={() => void copy()}><Copy size={20} /></IconButton>}<IconButton label="Download" onClick={() => void download()}><Download size={20} /></IconButton>{!video && <><IconButton label="Zoom in" onClick={() => setZoom((value) => Math.min(5, value * 1.25))}><ZoomIn size={20} /></IconButton><IconButton label="Zoom out" onClick={() => setZoom((value) => Math.max(.5, value * .8))}><ZoomOut size={20} /></IconButton><IconButton label="Rotate" onClick={() => setRotation((value) => (value + 90) % 360)}><RotateCw size={20} /></IconButton></>}<IconButton label="Close preview" onClick={onClose}><X size={20} /></IconButton></nav></header><div className={classes("media-preview-stage")} onClick={(event) => event.stopPropagation()}>{video ? <VideoPreviewPlayer source={source} /> : <><img src={selectedSource} alt={selectedName} draggable={false} onPointerDown={pointerDown} onPointerMove={pointerMove} onPointerUp={pointerUp} onPointerCancel={pointerUp} onWheel={(event) => { if (!event.ctrlKey) return; event.preventDefault(); setZoom((value) => Math.max(.5, Math.min(5, value * (event.deltaY < 0 ? 1.1 : .9)))); }} style={{ transform: `translate(${translation.x}px, ${translation.y}px) scale(${zoom}) rotate(${rotation}deg)` }} />{images.length > 1 && <><button type="button" className={classes("media-preview-page previous")} aria-label="Previous image" disabled={index === 0} onClick={() => select(index - 1)}>‹</button><button type="button" className={classes("media-preview-page next")} aria-label="Next image" disabled={index + 1 === images.length} onClick={() => select(index + 1)}>›</button></>}</>}</div></div>;
}

function VideoPreviewPlayer({ source }: { source: string }) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const wasPlayingRef = useRef(false);
  const lastAudibleVolumeRef = useRef(1);
  const hideControlsTimerRef = useRef<number | undefined>(undefined);
  const [playing, setPlaying] = useState(true);
  const [position, setPosition] = useState(0);
  const [buffered, setBuffered] = useState(0);
  const [dragPosition, setDragPosition] = useState<number>();
  const [showControls, setShowControls] = useState(true);
  const [volume, setVolume] = useState(1);
  const clearHideControlsTimer = useCallback(() => {
    if (hideControlsTimerRef.current !== undefined) window.clearTimeout(hideControlsTimerRef.current);
  }, []);
  const showControlsTemporarily = useCallback(() => {
    clearHideControlsTimer();
    setShowControls(true);
    hideControlsTimerRef.current = window.setTimeout(() => setShowControls(false), 3000);
  }, [clearHideControlsTimer]);
  useEffect(() => {
    showControlsTemporarily();
    return clearHideControlsTimer;
  }, [clearHideControlsTimer, showControlsTemporarily]);
  const percentAt = (clientX: number) => {
    const rect = barRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return 0;
    return Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  };
  const seek = (percent: number) => {
    const video = videoRef.current;
    if (video && Number.isFinite(video.duration) && video.duration > 0) video.currentTime = percent * video.duration;
  };
  const seekBy = (seconds: number) => {
    const video = videoRef.current;
    if (video && Number.isFinite(video.duration)) video.currentTime = Math.max(0, Math.min(video.duration, video.currentTime + seconds));
  };
  const setPlayerVolume = (next: number) => {
    const value = Math.max(0, Math.min(1, next));
    const video = videoRef.current;
    if (video) video.volume = value;
    if (value > 0) lastAudibleVolumeRef.current = value;
    setVolume(value);
  };
  const toggleMuteForKeyboard = useEffectEvent(() => setPlayerVolume(volume > 0 ? 0 : lastAudibleVolumeRef.current));
  const togglePlaybackForKeyboard = useEffectEvent(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) void video.play(); else video.pause();
    showControlsTemporarily();
  });
  const handleVideoKeyboard = useEffectEvent((event: globalThis.KeyboardEvent) => {
    if (event.key === ' ') { event.preventDefault(); togglePlaybackForKeyboard(); }
    if (event.key === 'ArrowLeft') { event.preventDefault(); seekBy(-15); showControlsTemporarily(); }
    if (event.key === 'ArrowRight') { event.preventDefault(); seekBy(15); showControlsTemporarily(); }
    if (event.key === 'ArrowUp') { event.preventDefault(); setPlayerVolume(volume + .1); showControlsTemporarily(); }
    if (event.key === 'ArrowDown') { event.preventDefault(); setPlayerVolume(volume - .1); showControlsTemporarily(); }
    if (event.key.toLowerCase() === 'm' && (event.metaKey || event.ctrlKey)) { event.preventDefault(); toggleMuteForKeyboard(); showControlsTemporarily(); }
  });
  const toggleMute = () => setPlayerVolume(volume > 0 ? 0 : lastAudibleVolumeRef.current);
  const togglePlayback = () => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) void video.play(); else video.pause();
    showControlsTemporarily();
  };
  const update = (video: HTMLVideoElement) => {
    if (video.duration > 0) {
      setPosition(video.currentTime / video.duration);
      const end = video.buffered.length ? video.buffered.end(video.buffered.length - 1) : 0;
      setBuffered(Math.min(1, end / video.duration));
    }
  };
  useEffect(() => {
    const keydown = (event: globalThis.KeyboardEvent) => handleVideoKeyboard(event);
    window.addEventListener('keydown', keydown);
    return () => window.removeEventListener('keydown', keydown);
  }, []);
  return <div className={classes("video-preview-player")}>
    <video ref={videoRef} src={source} autoPlay onPointerMove={showControlsTemporarily} onTimeUpdate={(event) => update(event.currentTarget)} onProgress={(event) => update(event.currentTarget)} onLoadedMetadata={(event) => { update(event.currentTarget); setPlayerVolume(event.currentTarget.volume); }} onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} />
    {showControls && <div className={classes("video-operation-bar")} onPointerMove={showControlsTemporarily}>
      <div className={classes("video-operation-row")}><span className={classes("video-volume")}><button type="button" aria-label={volume > 0 ? 'Mute video' : 'Unmute video'} onClick={toggleMute}>{volume > 0 ? <Volume2 size={20} /> : <VolumeX size={20} />}</button><input aria-label="Video volume" type="range" min="0" max="1" step=".01" value={volume} onChange={(event) => setPlayerVolume(Number(event.currentTarget.value))} /></span><button type="button" aria-label="Back 15 seconds" onClick={() => { seekBy(-15); showControlsTemporarily(); }}><Undo2 size={20} /></button><button type="button" className={classes("video-preview-toggle")} aria-label={playing ? 'Pause video' : 'Play video'} onClick={togglePlayback}>{playing ? <Pause size={20} /> : <Play size={20} />}</button><button type="button" aria-label="Forward 15 seconds" onClick={() => { seekBy(15); showControlsTemporarily(); }}><RotateCw size={20} /></button></div>
      <div className={classes("video-operation-row video-operation-progress")}><small>{formatDuration(String((videoRef.current?.currentTime || 0) * 1000))}</small><div ref={barRef} className={classes("video-progress")} role="slider" aria-label="Video progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round((dragPosition ?? position) * 100)} tabIndex={0} onPointerDown={(event) => { const video = videoRef.current; wasPlayingRef.current = Boolean(video && !video.paused); if (wasPlayingRef.current) video?.pause(); event.currentTarget.setPointerCapture(event.pointerId); setDragPosition(percentAt(event.clientX)); }} onPointerMove={(event) => { if (event.currentTarget.hasPointerCapture(event.pointerId)) setDragPosition(percentAt(event.clientX)); }} onPointerUp={(event) => { if (!event.currentTarget.hasPointerCapture(event.pointerId)) return; const next = percentAt(event.clientX); event.currentTarget.releasePointerCapture(event.pointerId); setDragPosition(undefined); seek(next); if (wasPlayingRef.current) void videoRef.current?.play(); showControlsTemporarily(); }} onPointerCancel={(event) => { if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId); setDragPosition(undefined); if (wasPlayingRef.current) void videoRef.current?.play(); }} onKeyDown={(event) => { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); seekBy(event.key === 'ArrowLeft' ? -15 : 15); showControlsTemporarily(); } }}><i className={classes("video-progress-background")} /><i className={classes("video-progress-buffered")} style={{ width: `${buffered * 100}%` }} /><i className={classes("video-progress-played")} style={{ width: `${(dragPosition ?? position) * 100}%` }} /><b style={{ left: `${(dragPosition ?? position) * 100}%` }} /></div><small>{formatDuration(String((videoRef.current?.duration || 0) * 1000))}</small></div>
    </div>}
  </div>;
}

function SpecialJsonMessage({ message }: { message: Message }) {
  try {
    if (message.category === 'APP_BUTTON_GROUP') {
      const actions = JSON.parse(message.content) as Array<{ label?: string; action?: string; color?: string }>;
      return <div className={classes("button-group-message")}>{actionButtonRows(actions).map((row, rowIndex) => <div className={classes("button-group-row")} key={rowIndex}>{row.map(({ action, index }) => <button type="button" key={`${action.label}-${index}`} onClick={() => void openAppCardAction(message.conversationId, action.action, action.label || '')} style={{ color: action.color || undefined }}><span>{action.label || 'Open'}</span>{actionButtonIndicator(action.action)}</button>)}</div>)}</div>;
    }
    if (message.category.endsWith('_LOCATION')) {
      const location = JSON.parse(message.content) as { latitude: number; longitude: number; name?: string; address?: string };
      return <LocationMessage latitude={location.latitude} longitude={location.longitude} address={location.address || ''} />;
    }
    if (message.category.endsWith('_TRANSCRIPT')) {
      const lines = JSON.parse(message.content) as Array<{ name?: string; content?: string; category?: string; type?: string; media_name?: string }>;
      return <TranscriptMessage transcriptId={message.messageId} lines={lines} />;
    }
  } catch { return <p className={classes("message-text")}>[{message.category}]</p>; }
  return null;
}

function actionButtonIndicator(raw?: string) {
  try {
    const uri = new URL(raw || '');
    const mixinHost = uri.hostname === 'mixin.one' || uri.hostname === 'www.mixin.one';
    const isSend = (uri.protocol === 'mixin:' && uri.hostname === 'send')
      || (mixinHost && uri.pathname.split('/')[1] === 'send');
    if (isSend && uri.searchParams.get('user')?.trim()) return <img className={classes("button-group-link-send")} src={linkSendUrl} alt="" />;
    const mixinActions = new Set(['codes', 'pay', 'users', 'transfer', 'device', 'send', 'address', 'withdrawal', 'apps', 'snapshots', 'conversations', 'multisigs', 'swap', 'markets', 'membership']);
    if ((uri.protocol === 'http:' || uri.protocol === 'https:') && !(mixinHost && mixinActions.has(uri.pathname.split('/')[1]))) return <img className={classes("button-group-external")} src={externalLinkUrl} alt="" />;
  } catch { /* Flutter treats malformed actions as ordinary buttons. */ }
  return undefined;
}

function TranscriptMessage({ transcriptId, lines }: { transcriptId: string; lines: Array<{ name?: string; content?: string; category?: string; type?: string; media_name?: string }> }) {
  const [open, setOpen] = useState(false);
  return <><button type="button" className={classes("transcript-message")} onClick={() => setOpen(true)}><header><strong>Transcript</strong><span>↗</span></header>{lines.slice(0, 4).map((line, index) => <p key={index}>{line.name || 'Unknown'}: {line.content || line.media_name || `[${line.category || line.type}]`}</p>)}</button>{open && <TranscriptDialog transcriptId={transcriptId} onClose={() => setOpen(false)} />}</>;
}

function TranscriptDialog({ transcriptId, onClose }: { transcriptId: string; onClose: () => void }) {
  const messages = useQuery({ queryKey: ['transcript-messages', transcriptId], queryFn: () => desktopApi.transcriptMessages(transcriptId) });
  return <div className={classes("modal-backdrop transcript-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("transcript-dialog")} role="dialog" aria-modal="true" aria-label="Transcript" onMouseDown={(event) => event.stopPropagation()}><header><IconButton label="Close transcript" onClick={onClose}><X size={20} /></IconButton><strong>Transcript</strong><span /></header><div className={classes("transcript-list")}>{messages.isPending && <div className={classes("chat-state")}>Loading transcript…</div>}{messages.isError && <div className={classes("chat-state is-error")}>Unable to load transcript</div>}{messages.data?.map((item) => <article className={classes(`transcript-row ${item.senderId === '' ? '' : 'has-sender'}`)} key={item.messageId}><small>{item.senderName}</small><MessageContent message={item} /></article>)}</div></section></div>;
}

function LocationMessage({ latitude, longitude, address }: { latitude: number; longitude: number; address: string }) {
  const zoom = 15;
  const center = mapTilePosition(latitude, longitude, zoom);
  const tiles = [-1, 0, 1].flatMap((row) => [-1, 0, 1].map((column) => ({
    x: (center.x + column + center.scale) % center.scale,
    y: Math.min(Math.max(center.y + row, 0), center.scale - 1),
    row,
    column,
  })));
  const href = address
    ? `https://www.google.com/maps/search/${encodeURIComponent(address)}/@${latitude},${longitude},17z?hl=zh-CN`
    : `https://www.google.com/maps/place/@${latitude},${longitude},17z?hl=zh-CN`;
  const tilesOffset = {
    transform: `translate(${130 - (256 + center.fractionX * 256)}px, ${90 - (256 + center.fractionY * 256)}px)`,
  };
  return <button type="button" className={classes("location-message")} aria-label="Open location in Google Maps" onClick={() => openExternal(href)}><span className={classes("location-tiles")} style={tilesOffset}>{tiles.map((tile) => <img key={`${tile.x}-${tile.y}`} src={`https://www.google.com/maps/vt/pb=!1m4!1m3!1i${zoom}!2i${tile.x}!3i${tile.y}!2m3!1e0!2sm!3i420120488!3m7!2sen!5e1105!12m4!1e68!2m2!1sset!2sRoadmap!4e0!5m1!1e0!23i4111425`} alt="" style={{ gridRow: tile.row + 2, gridColumn: tile.column + 2 }} />)}</span><img className={classes("location-pin")} src={locationMarkUrl} alt="" /></button>;
}

export function MessageContent({ message, onLocateQuote, onOpenUser, mentionNames = {}, currentUserId, audioPlaylist = [], imagePlaylist = [] }: { message: Message; onLocateQuote?: () => void; onOpenUser?: (userId: string) => void; mentionNames?: Record<string, string>; currentUserId?: string; audioPlaylist?: Message[]; imagePlaylist?: Message[] }) {
  const category = message.category;
  const unsupported = message.status.toUpperCase() === 'UNKNOWN'
    || hasInvalidSpecialPayload(message)
    || (category.endsWith('_IMAGE') && (message.mediaWidth == null || message.mediaHeight == null));
  if (unsupported) return <p className={classes("message-text unsupported-message")}>This type of message is not supported, please upgrade Mixin to the latest version.</p>;
  if (message.status.toUpperCase() === 'FAILED') return <p className={classes("message-text waiting-message")}>Waiting for {message.senderId === currentUserId ? 'your linked device' : message.senderName} to get online and establish an encrypted session. <button type="button" onClick={() => openExternal('https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p')}>Learn more</button></p>;
  if (category === 'SYSTEM_CONVERSATION') return <p className={classes("system-event-message")}>{systemConversationText(message, currentUserId)}</p>;
  if (category === 'MESSAGE_PIN') return <p className={classes("system-event-message")}>{message.senderName} pinned {pinPreview(message.content, mentionNames)}</p>;
  if (category === 'SECRET') return <button type="button" className={classes("secret-message")} onClick={() => openExternal('https://mixin.one/pages/1000007')}>Messages in this conversation are end-to-end encrypted. Tap for more info.</button>;
  if (category === 'STRANGER') return <StrangerMessage message={message} />;
  if (category === 'MESSAGE_RECALL') return <p className={classes("recalled-message")}>{message.senderId === currentUserId ? 'You deleted this message' : 'This message was deleted'}</p>;
  const imageUrl = resolveAssetUrl(message.mediaUrl || message.thumbUrl);
  const stickerUrl = resolveAssetUrl(message.stickerAssetUrl ?? message.mediaUrl);
  const onOpenIdentity = (identityNumber: string) => openMessageIdentity(identityNumber, onOpenUser);

  if (category === 'APP_BUTTON_GROUP' || category.endsWith('_LOCATION') || category.endsWith('_TRANSCRIPT')) return <SpecialJsonMessage message={message} />;

  if (category === 'APP_CARD') {
    try {
      const card = JSON.parse(message.content) as {
        icon_url?: string;
        cover_url?: string;
        cover?: { url?: string; width?: number; height?: number };
        title?: string;
        description?: string;
        action?: string;
        actions?: Array<{ label?: string; action?: string; color?: string }>;
      };
      const action = card.action?.trim() || '';
      const cover = card.cover_url?.trim() || card.cover?.url?.trim() || '';
      const coverRatio = card.cover_url ? 1 : Math.max((card.cover?.width || 0) / (card.cover?.height || 0), 1.5);
      const runAction = (value?: string) => openAppCardAction(message.conversationId, value, card.title || '');
      if (action) return <button type="button" className={classes("app-card-compact")} onClick={() => void runAction(action)}>
        <span className={classes("app-card-icon")}>{card.icon_url && <MixinImage source={card.icon_url} alt="" />}</span>
        <span className={classes("app-card-copy")}><strong>{card.title || ''}</strong><small>{(card.description || '').split('\n', 1)[0]}</small></span>
      </button>;
      return <div className={classes("app-card-actions")}>
        <div className={classes("app-card-actions-body")}>
          {cover && <span className={classes("app-card-cover")} style={{ aspectRatio: String(coverRatio) }}><MixinImage source={cover} alt="" /></span>}
          <strong>{card.title || ''}</strong>
          <SelectableMessageText className={classes("app-card-description")} content={card.description || ''} mentionNames={mentionNames} onOpenIdentity={onOpenIdentity} />
        </div>
        {card.actions?.length ? <div className={classes("app-card-action-list")}>{card.actions.map((item, index) => <button type="button" key={`${item.label}-${index}`} onClick={() => void runAction(item.action)} style={{ color: item.color || undefined }}>{item.label || ''}</button>)}</div> : null}
      </div>;
    } catch {
      return <p className={classes("message-text")}>[App card]</p>;
    }
  }
  if (category.endsWith('_AUDIO')) {
    return <AudioMessage message={message} currentUserId={currentUserId} playlist={audioPlaylist} />;
  }
  if (category === 'SYSTEM_SAFE_INSCRIPTION') {
    return <InscriptionMessage message={message} />;
  }
  if (category === 'SYSTEM_ACCOUNT_SNAPSHOT' || category === 'SYSTEM_SAFE_SNAPSHOT') {
    return <SnapshotMessage message={message} safe={category === 'SYSTEM_SAFE_SNAPSHOT'} />;
  }

  if (category.includes('STICKER') && stickerUrl) {
    return <StickerMessage stickerId={message.stickerId} source={stickerUrl} assetType={message.stickerAssetType} />;
  }
  if (category.includes('VIDEO') && message.mediaUrl) {
    return <VideoMessage message={message} source={resolveAssetUrl(message.mediaUrl)!} poster={resolveAssetUrl(message.thumbUrl)}  mentionNames={mentionNames} onOpenIdentity={onOpenIdentity} />;
  }
  if (category.includes('IMAGE') && imageUrl) {
    return <ImageMessage message={message} source={imageUrl}  mentionNames={mentionNames} imagePlaylist={imagePlaylist} onOpenIdentity={onOpenIdentity} />;
  }
  if (category.endsWith('_DATA')) {
    return (
      <div className={classes("file-message")}>
        <span><FileText size={22} /></span>
        <div><strong>{message.mediaName || 'Attachment'}</strong><small>{message.mediaMimeType || 'File'}</small></div>
      </div>
    );
  }
  if (category.endsWith('_POST')) {
    return <PostMessage message={message} />;
  }
  if (category.includes('CONTACT')) {
    const userId = message.sharedUserId || '';
    return (
      <button type="button" className={classes("contact-message")} disabled={!userId} onClick={() => userId && onOpenUser?.(userId)}>
        <Avatar name={message.sharedUserFullName || 'Contact'} url={message.sharedUserAvatarUrl} size={40} />
        <span><strong>{message.sharedUserFullName || 'Contact'}<Badges verified={Boolean(message.sharedUserIsVerified)} isBot={Boolean(message.sharedUserAppId)} membership={message.sharedUserMembership} /></strong><small>{message.sharedUserIdentityNumber || ''}</small></span>
      </button>
    );
  }
  return (
    <>
      {message.quoteMessageId && <QuotePreview raw={message.quoteContent} membership={message.quoteUserMembership} mentionNames={mentionNames} onLocate={onLocateQuote} />}
      <MessageText content={message.content || `[${category}]`} mentionNames={mentionNames} onOpenUser={onOpenUser} />
    </>
  );
}

async function openAppCardAction(conversationId: string, rawAction: string | undefined, title: string) {
  const action = rawAction?.trim() || '';
  if (!action) return;
  if (action.startsWith('input:')) {
    const content = action.slice('input:'.length).trim();
    if (content) await desktopApi.sendText(conversationId, content);
    return;
  }
  let url: URL;
  try { url = new URL(action); } catch { return; }
  if (url.protocol === 'mixin:' || url.hostname === 'mixin.one' || url.hostname === 'www.mixin.one') {
    window.dispatchEvent(new CustomEvent('mixin://open', { detail: action }));
    return;
  }
  await openBotWebview({ url: action, title, conversationId });
}

function StickerMessage({ stickerId, source, assetType }: { stickerId?: string | null; source: string; assetType?: string | null }) {
  const [detailOpen, setDetailOpen] = useState(false);
  const refresh = () => {
    if (!stickerId) return;
    queueMicrotask(() => {
      void desktopApi.refreshSticker(stickerId).catch((reason) => e('Unable to refresh sticker asset', reason));
    });
  };
  const image = assetType === 'json'
    ? <LottieSticker className={classes("sticker-message")} source={source} alt="Sticker" onError={refresh} />
    : <MixinImage className={classes("sticker-message")} source={source} alt="Sticker" fit="contain" normalizeGif={assetType?.toLowerCase() === 'gif'} onError={refresh} />;
  if (!stickerId) return image;
  return <>{<button type="button" className={classes("sticker-message-button")} onClick={() => setDetailOpen(true)}>{image}</button>}{detailOpen && <StickerDetailDialog stickerId={stickerId} onClose={() => setDetailOpen(false)} />}</>;
}

function pinPreview(raw: string, mentionNames: Record<string, string>) {
  try {
    const value = JSON.parse(raw) as { category?: string; content?: string };
    const category = value.category || '';
    let preview = value.content?.trim() || '';
    if (category.endsWith('_TEXT')) {
      for (const [id, name] of Object.entries(mentionNames)) preview = preview.replaceAll(`@${id}`, `@${name}`);
      return preview || 'a message';
    }
    if (category.endsWith('_POST')) return preview || 'Post';
    if (category === 'APP_BUTTON_GROUP') return JSON.parse(preview).map((item: { label?: string }) => `[${item.label || ''}]`).join('') || 'a message';
    if (category === 'APP_CARD') return `[${(JSON.parse(preview) as { title?: string }).title || 'Card'}]`;
    if (category.startsWith('WEBRTC') || category.startsWith('KRAKEN')) return 'Voice message';
    if (category === 'MESSAGE_RECALL') return '[This message was deleted]';
    if (category.includes('IMAGE')) return ': Photo';
    if (category.includes('VIDEO')) return ': Video';
    if (category.includes('AUDIO')) return ': Audio';
    if (category.includes('STICKER')) return ': Sticker';
    if (category.includes('CONTACT')) return ': Contact';
    if (category.endsWith('_DATA')) return ': File';
    return 'a message';
  } catch { return 'a message'; }
}

function systemConversationText(message: Message, currentUserId?: string) {
  const participant = message.participantId === currentUserId ? 'You' : message.participantFullName || '';
  const sender = message.senderId === currentUserId ? 'You' : message.senderName || '';
  switch (message.action?.toUpperCase()) {
    case 'JOIN': return `${participant} joined the group`;
    case 'EXIT': return `${participant} left the group`;
    case 'ADD': return `${sender} added ${participant}`;
    case 'REMOVE': return `${sender} removed ${participant}`;
    case 'CREATE': return `${sender} created this group`;
    case 'ROLE': return `${participant} is now an admin`;
    case 'EXPIRE': return message.expireIn == null ? `${sender} changed disappearing message settings` : message.expireIn <= 0 ? `${sender} disabled disappearing messages` : `${sender} set disappearing messages to ${formatSystemExpire(message.expireIn)}`;
    default: return message.content || 'This conversation was updated.';
  }
}

function formatSystemExpire(seconds: number) {
  if (seconds < 60) return `${seconds} seconds`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} minutes`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} hours`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)} days`;
  return `${Math.floor(seconds / 604800)} weeks`;
}

function StrangerMessage({ message }: { message: Message }) {
  const queryClient = useQueryClient();
  const [resolved, setResolved] = useState(['FRIEND', 'BLOCKED'].includes(message.senderRelationship?.toUpperCase() || ''));
  if (resolved) return null;
  const bot = Boolean(message.senderIsBot);
  async function action(kind: 'block' | 'contact' | 'home' | 'hi') {
    if (kind === 'block') await desktopApi.blockUser(message.senderId);
    if (kind === 'contact') await desktopApi.addContact(message.senderId, message.senderName);
    if (kind === 'hi') await desktopApi.sendText(message.conversationId, 'Hi');
    if (kind === 'home' && message.senderAppId) {
      const uri = await desktopApi.botHomeUri(message.senderAppId);
      if (uri) await openBotWebview({ url: uri, title: message.senderName, conversationId: message.conversationId });
    }
    if (kind === 'block' || kind === 'contact') setResolved(true);
    await queryClient.invalidateQueries({ queryKey: ['messages', message.conversationId] });
  }
  return <div className={classes("stranger-message")}><p>{bot ? 'This bot sent you a message.' : 'This user is not in your contacts.'}</p><div><button type="button" onClick={() => void action(bot ? 'home' : 'block')}>{bot ? 'Open Home Page' : 'Block'}</button><button type="button" onClick={() => void action(bot ? 'hi' : 'contact')}>{bot ? 'Say Hi' : 'Add Contact'}</button></div></div>;
}

type InscriptionContentMode = 'small' | 'large';

function InscriptionContent({ contentType, contentUrl, iconUrl, mode }: {
  contentType?: string | null;
  contentUrl?: string | null;
  iconUrl?: string | null;
  mode: InscriptionContentMode;
}) {
  const content = resolveAssetUrl(contentUrl);
  const icon = resolveAssetUrl(iconUrl);
  if (contentType?.startsWith('image') && content) return <InscriptionImageContent source={content} />;
  if (contentType?.startsWith('text') && content && icon) return <TextInscriptionContent contentUrl={content} iconUrl={icon} mode={mode} />;
  return <img className={classes("inscription-placeholder")} src={inscriptionPlaceholderUrl} alt="" />;
}

function InscriptionImageContent({ source }: { source: string }) {
  const [failed, setFailed] = useState(false);
  return failed ? <img className={classes("inscription-placeholder")} src={inscriptionPlaceholderUrl} alt="" /> : <img className={classes("inscription-image")} src={source} alt="" onError={() => setFailed(true)} />;
}

function TextInscriptionContent({ contentUrl, iconUrl, mode }: { contentUrl: string; iconUrl: string; mode: InscriptionContentMode }) {
  const [content, setContent] = useState('');
  const [iconFailed, setIconFailed] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    setContent('');
    void fetch(contentUrl, { signal: controller.signal })
      .then(async (response) => inscriptionDisplayContent(await response.text()))
      .then((value) => { if (!controller.signal.aborted) setContent(value); })
      .catch((reason) => e('Unable to load inscription content', reason));
    return () => controller.abort();
  }, [contentUrl]);
  return <span className={classes(`text-inscription-content text-inscription-${mode}`)} style={{ backgroundImage: `url(${inscriptionTextBackgroundUrl})` }}>
    <span className={classes("text-inscription-icon")}>{iconFailed ? <img src={collectionPlaceholderUrl} alt="" /> : <img src={iconUrl} alt="" onError={() => setIconFailed(true)} />}</span>
    <span className={classes("text-inscription-copy")}>{content}</span>
  </span>;
}

function InscriptionMessage({ message }: { message: Message }) {
  const [open, setOpen] = useState(false);
  const icon = resolveAssetUrl(message.inscriptionIconUrl);
  return <><button type="button" className={classes("inscription-message")} onClick={() => message.inscriptionHash && setOpen(true)}><span className={classes("inscription-preview")}><InscriptionContent contentType={message.inscriptionContentType} contentUrl={message.inscriptionContentUrl} iconUrl={message.inscriptionIconUrl} mode="small" /></span><span className={classes("inscription-info")}><strong>{message.inscriptionName || ''}</strong><small>{message.inscriptionSequence == null ? '' : `#${message.inscriptionSequence}`}</small><span className={classes("inscription-footer")}><span className={classes("inscription-colors")}>{inscriptionColors(message.inscriptionHash).map((color, index) => <i key={index} style={{ background: color }} />)}</span>{icon ? <img src={icon} alt="" /> : <img src={collectionPlaceholderUrl} alt="" />}</span></span></button>{open && <InscriptionDetailDialog message={message} onClose={() => setOpen(false)} />}</>;
}

function InscriptionDetailDialog({ message, onClose }: { message: Message; onClose: () => void }) {
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("snapshot-detail-dialog inscription-detail-dialog")} role="dialog" aria-modal="true" aria-label="Inscription detail" onMouseDown={(event) => event.stopPropagation()}><header><IconButton label="Close inscription detail" onClick={onClose}><X size={20} /></IconButton></header><div className={classes("inscription-detail-body")}><InscriptionContent contentType={message.inscriptionContentType} contentUrl={message.inscriptionContentUrl} iconUrl={message.inscriptionIconUrl} mode="large" /><dl><div><dt>Hash</dt><dd><span className={classes("inscription-detail-colors")}>{inscriptionColors(message.inscriptionHash).map((color, index) => <i key={index} style={{ background: color }} />)}</span><span>{message.inscriptionHash}</span></dd></div><div><dt>ID</dt><dd>#{message.inscriptionSequence ?? ''}</dd></div><div><dt>Collection</dt><dd>{message.inscriptionName}</dd></div></dl></div></section></div>;
}

function formatSnapshotAmount(value: string) {
  const [whole, fraction] = value.split('.');
  const formatted = Number(whole).toLocaleString(undefined, { maximumFractionDigits: 0 });
  return fraction ? `${formatted}.${fraction.slice(0, 8)}` : formatted;
}

function decodeSafeMemo(raw: string) {
  if (!raw || raw.length % 2 || !/^[\da-f]+$/i.test(raw)) return raw;
  try { return new TextDecoder().decode(Uint8Array.from(raw.match(/.{2}/g)!.map((byte) => Number.parseInt(byte, 16)))); } catch { return raw; }
}

function SnapshotMessage({ message, safe }: { message: Message; safe: boolean }) {
  const [open, setOpen] = useState(false);
  const data = snapshotMessageData(message);
  const amount = data.amount;
  const symbol = data.symbol;
  const memo = decodeSafeMemo(message.snapshotMemo || '');
  const icon = resolveAssetUrl(message.snapshotAssetIconUrl);
  const chainIcon = resolveAssetUrl(message.snapshotChainIconUrl);
  const width = amount.length < 10 ? 174 : amount.length < 15 ? 190 : amount.length < 25 ? 216 : 232;
  const canOpen = Boolean(data.id || message.inscriptionHash);
  const asset = <span className={classes("snapshot-asset-icon")}>{icon ? <img src={icon} alt="" /> : <i />}{chainIcon && <img className={classes("snapshot-chain-icon")} src={chainIcon} alt="" />}</span>;
  const card = safe ? <button type="button" className={classes("safe-snapshot-message")} style={{ width }} disabled={!canOpen} onClick={() => setOpen(true)}><span className={classes("safe-snapshot-art")} /><header>{asset}{symbol}</header><strong>{formatSnapshotAmount(amount)}</strong>{memo && <small>{memo}</small>}</button> : <button type="button" className={classes("account-snapshot-message")} disabled={!canOpen} onClick={() => setOpen(true)}>{asset}<span><strong>{formatSnapshotAmount(amount)}</strong><small>{symbol}</small></span></button>;
  return <>{card}{open && <SnapshotDetailDialog message={message} safe={safe} memo={memo} onClose={() => setOpen(false)} />}</>;
}

function SnapshotDetailDialog({ message, safe, memo, onClose }: { message: Message; safe: boolean; memo: string; onClose: () => void }) {
  const [detail, setDetail] = useState<import('@/shared/model/types').SnapshotDetail>();
  const bootstrap = useQuery({ queryKey: ['bootstrap'], queryFn: desktopApi.bootstrap, staleTime: Infinity });
  const data = snapshotMessageData(message);
  useEffect(() => {
    if (!data.id || message.inscriptionHash) return undefined;
    let disposed = false;
    void (safe ? desktopApi.safeSnapshotById(data.id) : desktopApi.snapshotById(data.id))
      .then((value) => { if (!disposed) setDetail(value); })
      .catch((reason) => e('Unable to load snapshot details', reason));
    return () => { disposed = true; };
  }, [data.id, message.inscriptionHash, safe]);
  return <SnapshotDetailPage snapshot={detail ?? snapshotDetailFromMessage(message, safe, memo)} fiatCurrency={bootstrap.data?.profile?.fiatCurrency || 'USD'} onClose={onClose} />;
}

function snapshotDetailFromMessage(message: Message, safe: boolean, memo: string): import('@/shared/model/types').SnapshotDetail {
  const data = snapshotMessageData(message);
  return {
    snapshotId: data.id || '',
    traceId: null,
    snapshotType: message.snapshotType || 'transfer',
    assetId: '',
    amount: data.amount,
    createdAtMillis: parseSnapshotCreatedAt(message.snapshotCreatedAt, message.createdAtMicros),
    opponentId: message.snapshotOpponentId,
    opponentName: message.snapshotOpponentId,
    transactionHash: message.snapshotTransactionHash,
    sender: null,
    receiver: null,
    memo,
    confirmations: null,
    snapshotHash: null,
    openingBalance: null,
    closingBalance: null,
    symbol: data.symbol,
    assetName: data.symbol,
    assetIconUrl: message.snapshotAssetIconUrl || '',
    chainIconUrl: message.snapshotChainIconUrl || '',
    assetConfirmations: 0,
    assetTag: null,
    currentUserName: '',
    isSafe: safe,
    priceUsd: null,
    fiatRate: null,
    tickerPriceUsd: null,
    depositHash: null,
    withdrawalHash: null,
    withdrawalReceiver: null,
  };
}

function parseSnapshotCreatedAt(value: string | null | undefined, fallbackMicros: number): number {
  if (!value) return fallbackMicros / 1000;
  const numeric = Number(value);
  if (Number.isFinite(numeric)) return numeric > 100_000_000_000_000 ? Math.floor(numeric / 1000) : numeric < 100_000_000_000 ? numeric * 1000 : numeric;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : fallbackMicros / 1000;
}

function snapshotMessageData(message: Message) {
  try {
    const parsed = JSON.parse(message.content) as { snapshot_id?: unknown; inscription_hash?: unknown; amount?: unknown; symbol?: unknown; asset_symbol?: unknown };
    return {
      id: message.snapshotId || message.inscriptionHash || stringValue(parsed.snapshot_id) || stringValue(parsed.inscription_hash),
      amount: message.snapshotAmount || stringValue(parsed.amount) || '',
      symbol: message.snapshotAssetSymbol || stringValue(parsed.symbol) || stringValue(parsed.asset_symbol) || '',
    };
  } catch {
    return { id: message.snapshotId || message.inscriptionHash, amount: message.snapshotAmount || '', symbol: message.snapshotAssetSymbol || '' };
  }
}

function stringValue(value: unknown) {
  return typeof value === 'string' || typeof value === 'number' ? String(value) : undefined;
}

function MessageText({ content, mentionNames, onOpenUser }: { content: string; mentionNames: Record<string, string>; onOpenUser?: (userId: string) => void }) {
  return <SelectableMessageText content={content} mentionNames={mentionNames} onOpenIdentity={(identityNumber) => openMessageIdentity(identityNumber, onOpenUser)} />;
}

function openMessageIdentity(identityNumber: string, onOpenUser?: (userId: string) => void) {
  if (!onOpenUser) return;
  void desktopApi.searchUser(identityNumber).then((profile) => onOpenUser(profile.userId)).catch((error: unknown) => showToastFailed(error));
}

function PostMessage({ message }: { message: Message }) {
  const [open, setOpen] = useState(false);
  const preview = message.content.split(/\r?\n/).slice(0, 10).join('\n').slice(0, 1024);
  return <><button type="button" className={classes("post-message")} aria-label="Open post" onClick={() => setOpen(true)}><span>↗</span><MarkdownColumn data={preview} /></button>{open && <PostPreviewDialog content={message.content} onClose={() => setOpen(false)} />}</>;
}

function PostPreviewDialog({ content, onClose }: { content: string; onClose: () => void }) {
  return <div className={classes("post-preview-dialog")} role="presentation" onMouseDown={onClose}><section role="dialog" aria-modal="true" aria-label="Post preview" onMouseDown={(event) => event.stopPropagation()}><header><span /><IconButton label="Close post preview" onClick={onClose}><X size={20} /></IconButton></header><main><MarkdownColumn data={content} /></main></section></div>;
}
