import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type KeyboardEvent,
} from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { open } from '@tauri-apps/plugin-dialog';
import {
  Bot,
  Eye,
  EyeOff,
  FileText,
  Image as ImageIcon,
  Lock,
  Mic,
  Paperclip,
  Pause,
  Pencil,
  Play,
  Plus,
  RotateCw,
  Search,
  Send,
  Smile,
  Trash2,
  X,
} from 'lucide-react';
import { desktopApi, resolveAssetUrl } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import { flutterMenuIcon } from '@/shared/ui/flutterMenuIcon';
import { showNativeContextMenu } from '@/shared/ui/nativeMenu';
import { useDesktopUi } from '@/state/desktopUi';
import type {
  Conversation,
  ConversationParticipant,
  Sticker,
  StickerAlbum,
  VoiceRecording,
} from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { IconButton } from '@/shared/ui/IconButton';
import { useEditableSelectionToolbar } from '@/shared/ui/AdaptiveSelectionToolbar';
import { CustomPopupMenuButton } from '@/shared/ui/CustomPopupMenu';
import {
  dismissToast,
  showToastFailed,
  showToastLoading,
  showToastSuccessful,
} from '@/shared/ui/toast';
import { ImageEditorDialog } from '@/features/chat/components/ImageEditorDialog';
import { PickerStickerAsset } from '@/features/chat/components/StickerAsset';
import { StickerStoreDialog } from '@/features/chat/components/StickerDetailDialog';
import {
  CHAT_COMPOSER_MAX_TEXT_LENGTH,
  attachmentDraft,
  attachmentPreviewMode,
  formatAttachmentBytes,
  resizeComposerTextArea,
  type AttachmentDraft,
  type AttachmentPreviewMode,
} from '@/features/chat/model/composer';
import { formatDuration, messageNameColor } from '@/features/chat/model/message';
import emojiRecentUrl from '@flutter-assets/images/emoji_recent.svg';
import emojiFaceUrl from '@flutter-assets/images/emoji_face.svg';
import emojiAnimalUrl from '@flutter-assets/images/emoji_animal.svg';
import emojiFoodUrl from '@flutter-assets/images/emoji_food.svg';
import emojiTravelUrl from '@flutter-assets/images/emoji_travel.svg';
import emojiSportsUrl from '@flutter-assets/images/emoji_sports.svg';
import emojiObjectsUrl from '@flutter-assets/images/emoji_objects.svg';
import emojiSymbolUrl from '@flutter-assets/images/emoji_symbol.svg';
import emojiFlagsUrl from '@flutter-assets/images/emoji_flags.svg';
import { classes } from '@/shared/styles/classes';

const giphyApiKey = import.meta.env.MIXIN_GIPHY_KEY as string;

const rawEmojiGroups = [
  { title: 'Recent', icon: emojiRecentUrl, values: [] as string[] },
  { title: 'Smileys & People', icon: emojiFaceUrl, values: [...'😀😃😄😁😆😅😂🤣😊😇🙂🙃😉😍🥰😘😎🤓🤔😐😑😶🙄😏😣😥😮🤐😯😪😫🥱😴😌🤗🤭🫢🫣🤫🤥😶‍🌫️😵‍💫🥳🤩😤😡😭😱😨🤯🥶🥵🤢🤮🤧🤠👋🤚🖐️✋🖖👌🤌🤏✌️🤞🫰🤟🤘🤙👈👉👆👇☝️👍👎✊👊🤝🙏👏🙌🫶💪🫡❤️🧡💛💚💙💜🖤🤍🤎💔❤️‍🔥❤️‍🩹💕💖💗💓💞💘💝💟'], },
  { title: 'Animals & Nature', icon: emojiAnimalUrl, values: [...'🐶🐱🐭🐹🐰🦊🐻🐼🐨🐯🦁🐮🐷🐸🐵🙈🙉🙊🐔🐧🐦🦄🐝🦋🐌🐞🐢🐍🦎🦖🦕🐙🦑🦀🐠🐬🐳🦈🌸🌹🌺🌻🌼🌷🌱🌲🌳🌴🌵🍀☘️🌿🍃🍂🍁🌍🌙⭐🌈🔥💧❄️'], },
  { title: 'Food & Drink', icon: emojiFoodUrl, values: [...'🍏🍎🍐🍊🍋🍌🍉🍇🍓🫐🍈🍒🍑🥭🍍🥥🥝🍅🥑🥕🌽🌶️🥒🥬🥦🧄🧅🍄🥜🌰🍞🥐🥨🧀🥚🍳🥞🥓🥩🍗🍔🍟🍕🌭🥪🌮🌯🥗🍝🍜🍣🍱🥟🍤🍙🍚🍛🍰🎂🍪🍩🍫🍿🍭☕🍵🥤🍺🍻🍷🍸🍹'], },
  { title: 'Travel & Places', icon: emojiTravelUrl, values: [...'🚗🚕🚙🚌🚎🏎️🚓🚑🚒🚐🛻🚚🚜🏍️🚲🛴🛵🚄🚅🚆🚇🚉✈️🛫🛬🚀🛸🚁🛶⛵🚤🛳️⛴️🚢⚓🏖️🏝️🏔️⛰️🌋🗻🏕️🏠🏡🏢🏣🏤🏥🏦🏨🏫🏬🏭🏰🗽⛲🎡🎢🎠⛱️🌅🌄🌇🌃🌉🌌'], },
  { title: 'Activity', icon: emojiSportsUrl, values: [...'⚽🏀🏈⚾🥎🎾🏐🏉🥏🎱🏓🏸🏒🏑🥍🏏⛳🏹🎣🤿🥊🥋🎽🛹🛼🛷⛸️🥌🎿⛷️🏂🪂🏋️🤼🤸⛹️🤺🤾🏌️🏇🧘🏄🏊🤽🚣🧗🚵🚴🏆🥇🥈🥉🏅🎖️🎗️🎫🎪🎭🎨🎬🎤🎧🎼🎹🥁🎷🎺🎸🪕🎻🎲♟️🎯🎳🎮🎰🧩'], },
  { title: 'Objects', icon: emojiObjectsUrl, values: [...'⌚📱💻⌨️🖥️🖨️🖱️💽💾💿📀📼📷📸📹🎥📺📻🎙️⏰⌛⏳📡🔋🔌💡🔦🕯️🧯🛢️💸💵💴💶💷🪙💰💳🧾💎⚖️🧰🔧🔨⚒️🛠️⛏️🔩⚙️🧱⛓️🔫💣🧨🔪🗡️🛡️🚬⚰️⚱️🏺🔮📿🧿💈⚗️🔭🔬🩺💊💉🩹🩼🩻🧬🧪🧫'], },
  { title: 'Symbols', icon: emojiSymbolUrl, values: [...'💯💢💥💫💦💨🕳️💬👁️‍🗨️🗨️🗯️💤💮♨️💈🛑🕉️✡️☸️☯️✝️☦️☪️☮️🕎🔯♈♉♊♋♌♍♎♏♐♑♒♓⛎🔀🔁🔂▶️⏩⏭️⏯️◀️⏪⏮️🔼⏫🔽⏬⏸️⏹️⏺️⏏️🎦🔅🔆📶📳📴♀️♂️⚧️✖️➕➖➗♾️‼️⁉️❓❔❕❗〰️💱💲⚕️♻️✅❇️✳️❎🌐💠Ⓜ️🅰️🅱️🆎🆑🅾️🆘⛔📛🚫'], },
  { title: 'Flags', icon: emojiFlagsUrl, values: [...'🏁🚩🎌🏴🏳️🏳️‍🌈🏳️‍⚧️🇨🇳🇭🇰🇲🇴🇹🇼🇺🇸🇨🇦🇲🇽🇧🇷🇦🇷🇬🇧🇫🇷🇩🇪🇮🇹🇪🇸🇵🇹🇳🇱🇧🇪🇨🇭🇦🇹🇸🇪🇳🇴🇩🇰🇫🇮🇵🇱🇺🇦🇷🇺🇹🇷🇸🇦🇮🇳🇵🇰🇯🇵🇰🇷🇸🇬🇹🇭🇻🇳🇮🇩🇵🇭🇦🇺🇳🇿🇿🇦🇪🇬🇳🇬🇰🇪🇺🇳🇪🇺'], },
];

const emojiSegmenter = new (Intl as typeof Intl & { Segmenter: new (locales?: string | string[], options?: { granularity: 'grapheme' }) => { segment: (input: string) => Iterable<{ segment: string }> } }).Segmenter(undefined, { granularity: 'grapheme' });
const emojiGroups = rawEmojiGroups.map((group) => ({ ...group, values: [...emojiSegmenter.segment(group.values.join(''))].map((item) => item.segment) }));

interface GiphyGif {
  previewUrl: string;
  url: string;
  width?: number;
  height?: number;
}

function GiphyPage({ onSelected }: { onSelected: (gif: GiphyGif) => void }) {
  const [input, setInput] = useState('');
  const [query, setQuery] = useState('');
  const [gifs, setGifs] = useState<GiphyGif[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const offsetRef = useRef(0);
  const loadingRef = useRef(false);
  const hasMoreRef = useRef(true);
  const generationRef = useRef(0);

  useEffect(() => {
    const timeout = window.setTimeout(() => setQuery(input.trim()), 1000);
    return () => window.clearTimeout(timeout);
  }, [input]);
  useEffect(() => {
    let cancelled = false;
    const generation = ++generationRef.current;
    offsetRef.current = 0;
    hasMoreRef.current = true;
    setGifs([]);
    setHasMore(true);
    async function load() {
      loadingRef.current = true;
      setLoading(true);
      try {
        const endpoint = query ? 'gifs/search' : 'gifs/trending';
        const params = new URLSearchParams({ limit: '51', offset: '0', api_key: giphyApiKey });
        if (query) params.set('q', query);
        const response = await fetch(`https://api.giphy.com/v1/${endpoint}?${params}`);
        if (!response.ok) throw new Error(`GIPHY HTTP ${response.status}`);
        const data = (await response.json() as { data?: Array<{ images?: { fixed_width_downsampled?: Record<string, unknown>; fixed_width?: Record<string, unknown> } }> }).data ?? [];
        const page = data.flatMap((item): GiphyGif[] => {
          const preview = item.images?.fixed_width_downsampled;
          const send = item.images?.fixed_width;
          const previewUrl = String(preview?.url ?? '');
          const url = String(send?.url ?? '');
          if (!previewUrl || !url) return [];
          const width = Number(send?.width);
          const height = Number(send?.height);
          return [{ previewUrl, url, width: Number.isFinite(width) ? width : undefined, height: Number.isFinite(height) ? height : undefined }];
        });
        if (!cancelled && generation === generationRef.current) {
          offsetRef.current = page.length;
          hasMoreRef.current = page.length >= 51;
          setGifs(page);
          setHasMore(page.length >= 51);
        }
      } catch {
        // Keep Flutter's loading placeholder when GIPHY is unavailable.
      } finally {
        loadingRef.current = false;
        if (!cancelled) setLoading(false);
      }
    }
    void load();
    return () => { cancelled = true; };
  }, [query]);
  async function loadMore() {
    if (loadingRef.current || !hasMoreRef.current) return;
    const generation = generationRef.current;
    loadingRef.current = true;
    setLoading(true);
    try {
      const endpoint = query ? 'gifs/search' : 'gifs/trending';
      const params = new URLSearchParams({ limit: '51', offset: String(offsetRef.current), api_key: giphyApiKey });
      if (query) params.set('q', query);
      const response = await fetch(`https://api.giphy.com/v1/${endpoint}?${params}`);
      if (!response.ok) throw new Error(`GIPHY HTTP ${response.status}`);
      const data = (await response.json() as { data?: Array<{ images?: { fixed_width_downsampled?: Record<string, unknown>; fixed_width?: Record<string, unknown> } }> }).data ?? [];
      const page = data.flatMap((item): GiphyGif[] => {
        const preview = item.images?.fixed_width_downsampled;
        const send = item.images?.fixed_width;
        const previewUrl = String(preview?.url ?? ''); const url = String(send?.url ?? '');
        if (!previewUrl || !url) return [];
        const width = Number(send?.width); const height = Number(send?.height);
        return [{ previewUrl, url, width: Number.isFinite(width) ? width : undefined, height: Number.isFinite(height) ? height : undefined }];
      });
      if (generation !== generationRef.current) return;
      offsetRef.current += page.length;
      hasMoreRef.current = page.length >= 51;
      setGifs((current) => [...current, ...page]);
      setHasMore(page.length >= 51);
    } catch {
      // Keep the already-loaded GIFs visible on paging errors.
    } finally {
      if (generation === generationRef.current) {
        loadingRef.current = false;
        setLoading(false);
      }
    }
  }
  return <div className={classes("giphy-page")}><label><Search size={18} /><input value={input} onChange={(event) => setInput(event.target.value)} placeholder="Search" /></label><div className={classes("giphy-grid")} onScroll={(event) => { const element = event.currentTarget; if (element.scrollTop + element.clientHeight >= element.scrollHeight) void loadMore(); }}>{gifs.length === 0 && loading ? <span className={classes("giphy-loading")} /> : gifs.map((gif) => <button type="button" key={gif.url} onClick={() => onSelected(gif)}><img src={gif.previewUrl} alt="" /></button>)}{loading && gifs.length > 0 && <span className={classes("giphy-loading")} />}{!hasMore && <span />}</div></div>;
}

export function ChatComposer({ conversation, currentUserId, droppedPaths, onDroppedPathsHandled }: { conversation: Conversation; currentUserId: string; droppedPaths?: readonly string[]; onDroppedPathsHandled?: () => void }) {
  const queryClient = useQueryClient();
  const encryption = useQuery({ queryKey: ['conversation-encrypted', conversation.conversationId], queryFn: () => desktopApi.conversationIsEncrypted(conversation.conversationId) });
  const storedDraft = useDesktopUi((state) => state.drafts[conversation.conversationId]);
  const setStoredDraft = useDesktopUi((state) => state.setDraft);
  const quote = useDesktopUi((state) => state.quote);
  const setQuote = useDesktopUi((state) => state.setQuote);
  const [value, setValue] = useState(storedDraft ?? conversation.draft);
  const [attachmentPending, setAttachmentPending] = useState(false);
  const [attachmentError, setAttachmentError] = useState('');
  const [attachmentDrafts, setAttachmentDrafts] = useState<AttachmentDraft[]>();
  const [picker, setPicker] = useState<'none' | 'stickers' | 'contacts'>('none');
  const [recentStickers, setRecentStickers] = useState<Sticker[]>([]);
  const [personalStickers, setPersonalStickers] = useState<Sticker[]>([]);
  const [stickerAlbums, setStickerAlbums] = useState<StickerAlbum[]>([]);
  const [storeAlbums, setStoreAlbums] = useState<StickerAlbum[]>([]);
  const [storeAlbumStickers, setStoreAlbumStickers] = useState<Record<string, Sticker[]>>({});
  const [storeOpen, setStoreOpen] = useState(false);
  const [stickerDraftPath, setStickerDraftPath] = useState<string>();
  const [albumStickers, setAlbumStickers] = useState<Record<string, Sticker[]>>({});
  const [stickerTab, setStickerTab] = useState('recent');
  const [recentEmojis, setRecentEmojis] = useState<string[]>([]);
  const [emojiGroup, setEmojiGroup] = useState(0);
  const [contacts, setContacts] = useState<import('@/shared/model/types').UserProfile[]>([]);
  const [recording, setRecording] = useState<'recording' | VoiceRecording>();
  const [recordingSeconds, setRecordingSeconds] = useState(0);
  const [mentionKeyword, setMentionKeyword] = useState<string>();
  const [mentionIndex, setMentionIndex] = useState(0);
  const recordingStartedRef = useRef(0);
  const recordingTimerRef = useRef<number | undefined>(undefined);
  const previewAudioRef = useRef<HTMLAudioElement>(null);
  const [previewPlaying, setPreviewPlaying] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const selectionToolbar = useEditableSelectionToolbar(textareaRef);
  const quoteColor = quote ? messageNameColor(quote.senderId) : undefined;
  const quoteSource = quote ? resolveAssetUrl(quote.category.includes('STICKER') ? quote.stickerAssetUrl : quote.mediaUrl || quote.thumbUrl) : undefined;
  const mentionEnabled = conversation.category === 'GROUP' || conversation.isBot || conversation.isBotGroup;
  const participants = useQuery({
    queryKey: ['conversation-participants', conversation.conversationId],
    queryFn: () => desktopApi.conversationParticipants(conversation.conversationId),
    enabled: mentionEnabled,
  });
  const mentionCandidates = useMemo(() => {
    if (mentionKeyword === undefined) return [];
    const keyword = mentionKeyword.toLocaleLowerCase();
    return (participants.data ?? []).filter((participant) =>
      participant.userId !== currentUserId
      && (keyword.length === 0
        || participant.fullName.toLocaleLowerCase().includes(keyword)
        || participant.identityNumber.includes(keyword)),
    );
  }, [currentUserId, mentionKeyword, participants.data]);
  const send = useMutation({
    mutationFn: ({ content, silent }: { content: string; silent: boolean }) => desktopApi.sendText(conversation.conversationId, content, quote?.messageId, silent),
    onSuccess: async () => {
      setValue('');
      setStoredDraft(conversation.conversationId, '');
      setQuote();
      await desktopApi.updateDraft(conversation.conversationId, '');
      await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] });
    },
    onError: (error) => showToastFailed(error),
  });

  useEffect(() => {
    setValue(useDesktopUi.getState().drafts[conversation.conversationId] ?? conversation.draft);
    requestAnimationFrame(() => textareaRef.current?.focus({ preventScroll: true }));
  }, [conversation.conversationId, conversation.draft]);

  useEffect(() => {
    if (!droppedPaths?.length) return;
    let cancelled = false;
    void prepareAttachments([...droppedPaths]).finally(() => {
      if (!cancelled) onDroppedPathsHandled?.();
    });
    return () => { cancelled = true; };
  // prepareAttachment only relies on stable state setters.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedPaths]);

  useEffect(() => {
    setMentionKeyword(undefined);
    setMentionIndex(0);
  }, [conversation.conversationId]);

  useEffect(() => {
    setMentionIndex((index) => Math.min(index, Math.max(mentionCandidates.length - 1, 0)));
  }, [mentionCandidates.length]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      if (value !== conversation.draft) {
        void desktopApi.updateDraft(conversation.conversationId, value);
      }
    }, 500);
    return () => window.clearTimeout(timeout);
  }, [conversation.conversationId, conversation.draft, value]);

  useLayoutEffect(() => {
    if (textareaRef.current) resizeComposerTextArea(textareaRef.current);
  }, [value]);

  useEffect(() => () => {
    clearRecordingTimer();
    previewAudioRef.current?.pause();
  }, []);

  function submit(event?: FormEvent) {
    event?.preventDefault();
    const content = value.trim();
    if (!content || send.isPending) return;
    send.mutate({ content, silent: false });
  }

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (mentionCandidates.length > 0) {
      if (event.key === 'ArrowDown' || event.key === 'Tab' || (event.ctrlKey && event.key.toLowerCase() === 'n')) {
        event.preventDefault();
        setMentionIndex((index) => Math.min(index + 1, mentionCandidates.length - 1));
        return;
      }
      if (event.key === 'ArrowUp' || (event.ctrlKey && event.key.toLowerCase() === 'p')) {
        event.preventDefault();
        setMentionIndex((index) => Math.max(index - 1, 0));
        return;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        selectMention(mentionCandidates[mentionIndex]);
        return;
      }
    }
    if (event.key === 'Escape') {
      setMentionKeyword(undefined);
      setQuote();
      return;
    }
    if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      submit();
    }
  }

  function updateValue(next: string) {
    const draft = next.slice(0, CHAT_COMPOSER_MAX_TEXT_LENGTH);
    setValue(draft);
    setStoredDraft(conversation.conversationId, draft);
  }

  function updateMentionKeyword(next: string, selection: number) {
    if (!mentionEnabled) {
      setMentionKeyword(undefined);
      return;
    }
    const match = /@(\S*)$/.exec(next.slice(0, selection));
    setMentionKeyword(match?.[1]);
    setMentionIndex(0);
  }

  function selectMention(participant: ConversationParticipant | undefined) {
    if (!participant) return;
    const selection = textareaRef.current?.selectionStart ?? value.length;
    const before = value.slice(0, selection).replace(/@(\S*)$/, `@${participant.identityNumber} `);
    const next = before + value.slice(selection);
    updateValue(next);
    setMentionKeyword(undefined);
    requestAnimationFrame(() => {
      textareaRef.current?.focus({ preventScroll: true });
      textareaRef.current?.setSelectionRange(before.length, before.length);
    });
  }

  async function prepareAttachments(paths: string[]) {
    try {
      const files = await desktopApi.attachmentFileInfo([...new Set(paths)]);
      setAttachmentDrafts(files.map(attachmentDraft));
    } catch (error) {
      e('Unable to read attachment', error);
      setAttachmentError(error instanceof Error ? error.message : 'Unable to read attachment');
    }
  }

  async function chooseAttachment(imagesOnly: boolean) {
    if (attachmentPending) return;
    const selected = await open({
      multiple: true,
      directory: false,
      filters: imagesOnly ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'heic'] }] : undefined,
    });
    if (!selected) return;
    await prepareAttachments(Array.isArray(selected) ? selected : [selected]);
  }

  async function sendAttachments(drafts: AttachmentDraft[], mode: AttachmentPreviewMode, caption: string, zipPassword: string, silent: boolean) {
    setAttachmentPending(true);
    setAttachmentError('');
    showToastLoading();
    try {
      const outgoing = mode === 'zip'
        ? [attachmentDraft({ path: await desktopApi.createAttachmentArchive(drafts.map((draft) => draft.path), zipPassword || undefined), name: 'Archive.zip', size: 0 })]
        : drafts;
      for (const draft of outgoing) {
        const metadata = await attachmentMetadata(draft.path, draft.kind);
        if ((draft.kind === 'IMAGE' || draft.kind === 'VIDEO') && !metadata) throw new Error('Unable to read media metadata');
        await desktopApi.sendAttachment(
          conversation.conversationId,
          draft.path,
          draft.kind,
          draft.mimeType,
          draft.name,
          metadata?.width,
          metadata?.height,
          metadata?.durationMillis,
          draft.kind === 'VIDEO' ? 'L1GIo.]day]K-;jsfQjsRjfQj[fQ' : undefined,
          mode === 'zip' || drafts.length !== 1 || draft.kind !== 'IMAGE' ? undefined : caption.trim() || undefined,
          quote?.messageId,
          silent,
        );
      }
      setQuote();
      setAttachmentDrafts(undefined);
      await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] });
      dismissToast();
      showToastSuccessful();
    } catch (error) {
      e('Unable to send attachments', error);
      setAttachmentError(error instanceof Error ? error.message : 'Attachment failed to send');
      dismissToast();
      showToastFailed(error);
    } finally {
      setAttachmentPending(false);
    }
  }

  async function openStickers() {
    if (picker === 'stickers') { setPicker('none'); return; }
    const [recent, personal, albums, storedRecentEmojis] = await Promise.all([
      desktopApi.recentStickers(),
      desktopApi.personalStickers(),
      desktopApi.stickerAlbums(),
      desktopApi.setting('recent_used_emoji'),
    ]);
    setRecentStickers(recent);
    setPersonalStickers(personal);
    setStickerAlbums(albums);
    try { setRecentEmojis(storedRecentEmojis ? JSON.parse(storedRecentEmojis) as string[] : []); } catch { setRecentEmojis([]); }
    setStickerTab('recent');
    setPicker('stickers');
  }

  async function selectStickerTab(tab: string) {
    setStickerTab(tab);
    if (tab === 'store' && storeAlbums.length === 0) {
      const albums = await desktopApi.stickerStoreAlbums();
      const entries = await Promise.all(albums.map(async (album) => [album.albumId, await desktopApi.albumStickers(album.albumId)] as const));
      setStoreAlbums(albums);
      setStoreAlbumStickers(Object.fromEntries(entries));
      return;
    }
    if (!tab.startsWith('album:')) return;
    const albumId = tab.slice('album:'.length);
    if (albumStickers[albumId]) return;
    const stickers = await desktopApi.albumStickers(albumId);
    setAlbumStickers((current) => ({ ...current, [albumId]: stickers }));
  }

  async function openStickerStore() {
    setPicker('none');
    setStoreOpen(true);
    if (storeAlbums.length > 0) return;
    const albums = await desktopApi.stickerStoreAlbums();
    const entries = await Promise.all(albums.map(async (album) => [album.albumId, await desktopApi.albumStickers(album.albumId)] as const));
    setStoreAlbums(albums);
    setStoreAlbumStickers(Object.fromEntries(entries));
  }

  async function toggleStoreAlbum(album: StickerAlbum) {
    const added = !album.added;
    setStoreAlbums((current) => current.map((item) => item.albumId === album.albumId ? { ...item, added } : item));
    try {
      await desktopApi.setStickerAlbumAdded(album.albumId, added);
      setStickerAlbums(await desktopApi.stickerAlbums());
    } catch (error) {
      setStoreAlbums((current) => current.map((item) => item.albumId === album.albumId ? { ...item, added: !added } : item));
      showToastFailed(error);
    }
  }

  async function addSticker() {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Stickers', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    });
    if (!selected) return;
    setStickerDraftPath(selected);
  }

  async function saveSticker() {
    if (!stickerDraftPath) return;
    showToastLoading();
    try {
      await desktopApi.addStickerFromPath(stickerDraftPath);
      setPersonalStickers(await desktopApi.personalStickers());
      setStickerTab('personal');
      setStickerDraftPath(undefined);
      dismissToast();
      showToastSuccessful();
    } catch (error) {
      dismissToast();
      showToastFailed(error);
    }
  }

  async function removeSticker(stickerId: string) {
    await desktopApi.removeSticker(stickerId);
    setPersonalStickers((current) => current.filter((item) => item.stickerId !== stickerId));
  }

  function insertEmoji(emoji: string) {
    const start = textareaRef.current?.selectionStart ?? value.length;
    const end = textareaRef.current?.selectionEnd ?? start;
    const next = `${value.slice(0, start)}${emoji}${value.slice(end)}`;
    updateValue(next);
    const caret = start + emoji.length;
    requestAnimationFrame(() => {
      textareaRef.current?.focus({ preventScroll: true });
      textareaRef.current?.setSelectionRange(caret, caret);
    });
    const recent = [emoji, ...recentEmojis.filter((item) => item !== emoji)].slice(0, 35);
    setRecentEmojis(recent);
    void desktopApi.setSetting('recent_used_emoji', JSON.stringify(recent));
  }

  async function openContacts() {
    if (picker === 'contacts') { setPicker('none'); return; }
    setContacts(await desktopApi.selectableUsers());
    setPicker('contacts');
  }

  async function sendSticker(stickerId: string) {
    try {
      await desktopApi.sendSticker(conversation.conversationId, stickerId);
      setPicker('none');
      await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] });
    } catch (error) {
      showToastFailed(error);
    }
  }

  async function sendGif(gif: GiphyGif) {
    try {
      await desktopApi.sendRemoteImage(conversation.conversationId, gif.url, gif.previewUrl, gif.width, gif.height);
      await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] });
    } catch (error) {
      showToastFailed(error);
    }
  }

  async function sendContact(userId: string) {
    await desktopApi.sendContact(conversation.conversationId, userId, quote?.messageId);
    setQuote(); setPicker('none');
    await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] });
  }

  async function startRecording() {
    if (recording) return;
    await desktopApi.startVoiceRecording();
    recordingStartedRef.current = Date.now();
    setRecordingSeconds(0); setRecording('recording');
    recordingTimerRef.current = window.setInterval(() => setRecordingSeconds(Math.floor((Date.now() - recordingStartedRef.current) / 1000)), 250);
  }

  function clearRecordingTimer() {
    if (recordingTimerRef.current !== undefined) window.clearInterval(recordingTimerRef.current);
    recordingTimerRef.current = undefined;
  }

  async function cancelRecording() {
    clearRecordingTimer();
    setRecording(undefined); setPreviewPlaying(false);
    await desktopApi.cancelVoiceRecording();
  }

  async function stopRecording() {
    if (recording !== 'recording') return;
    clearRecordingTimer();
    setRecording(await desktopApi.stopVoiceRecording());
  }

  async function retryRecording() {
    if (recording !== 'recording') await desktopApi.cancelVoiceRecording();
    setRecording(undefined); setPreviewPlaying(false);
    await startRecording();
  }

  async function sendRecording() {
    const completed = recording === 'recording' ? await desktopApi.stopVoiceRecording() : recording;
    clearRecordingTimer();
    if (!completed) return;
    setRecording(undefined); setPreviewPlaying(false);
    await desktopApi.sendVoiceRecording(conversation.conversationId, completed, quote?.messageId);
    setQuote();
    await queryClient.invalidateQueries({ queryKey: ['messages', conversation.conversationId] });
  }

  return (
    <form className={classes(`composer${quote ? ' has-quote' : ''}`)} onSubmit={submit}>
      {mentionCandidates.length > 0 && <div className={classes("composer-mention-panel")}>{mentionCandidates.map((participant, index) => <button type="button" className={classes(index === mentionIndex ? 'is-selected' : '')} key={participant.userId} onMouseDown={(event) => event.preventDefault()} onClick={() => selectMention(participant)}><Avatar name={participant.fullName} url={participant.avatarUrl} size={32} /><span><strong>{participant.fullName}</strong><small>{participant.identityNumber}</small></span></button>)}</div>}
      {picker !== 'none' && <div className={classes(`composer-picker${picker === 'stickers' ? ' sticker-picker' : ''}`)}>{picker === 'stickers' ? <div className={classes("sticker-picker-content")}>
        <div className={classes("sticker-page")}>
          {stickerTab === 'emoji' && <div className={classes("emoji-picker")}><nav>{emojiGroups.map((group, index) => <button type="button" key={group.title} className={classes(emojiGroup === index ? 'is-selected' : '')} onClick={() => setEmojiGroup(index)} title={group.title}><img src={group.icon} alt={group.title} /></button>)}</nav><div className={classes("emoji-picker-body")}>{emojiGroup > 0 && <h3>{emojiGroups[emojiGroup].title}</h3>}<div>{(emojiGroup === 0 ? recentEmojis : emojiGroups[emojiGroup].values).map((emoji) => <button key={emoji} type="button" onClick={() => insertEmoji(emoji)}>{emoji}</button>)}</div></div></div>}
          {stickerTab === 'gif' && <GiphyPage onSelected={(gif) => void sendGif(gif)} />}
          {stickerTab !== 'emoji' && stickerTab !== 'gif' && <div className={classes("sticker-grid")}>{stickerTab === 'personal' && <button type="button" className={classes("add-sticker")} onClick={() => void addSticker()} aria-label="Add sticker"><Plus size={48} /></button>}{(stickerTab === 'recent' ? recentStickers : stickerTab === 'personal' ? personalStickers : albumStickers[stickerTab.slice('album:'.length)] ?? []).map((sticker) => {
            return <button type="button" key={sticker.stickerId} onClick={() => void sendSticker(sticker.stickerId)} onContextMenu={(event) => void showNativeContextMenu(event, [{ type: 'item', id: 'delete-personal-sticker', text: 'Delete', icon: flutterMenuIcon('delete'), action: () => removeSticker(sticker.stickerId) }])}><PickerStickerAsset sticker={sticker} /></button>;
          })}</div>}
        </div>
        <div className={classes("sticker-tabs")}>
          <button type="button" onClick={() => void openStickerStore()} aria-label="Sticker store"><Bot size={22} /></button>
          <button type="button" className={classes(stickerTab === 'emoji' ? 'is-selected' : '')} onClick={() => void selectStickerTab('emoji')} aria-label="Emoji">😀</button>
          <button type="button" className={classes(stickerTab === 'recent' ? 'is-selected' : '')} onClick={() => void selectStickerTab('recent')} aria-label="Recent">◷</button>
          <button type="button" className={classes(stickerTab === 'personal' ? 'is-selected' : '')} onClick={() => void selectStickerTab('personal')} aria-label="Personal">★</button>
          {giphyApiKey && <button type="button" className={classes(stickerTab === 'gif' ? 'is-selected' : '')} onClick={() => void selectStickerTab('gif')} aria-label="GIF">GIF</button>}
          {stickerAlbums.map((album) => <button type="button" key={album.albumId} className={classes(stickerTab === `album:${album.albumId}` ? 'is-selected' : '')} onClick={() => void selectStickerTab(`album:${album.albumId}`)} title={album.name}><PickerStickerAsset sticker={{ stickerId: album.albumId, albumId: null, name: album.name, assetUrl: album.iconUrl, assetWidth: 24, assetHeight: 24, assetType: album.iconUrl.toLowerCase().endsWith('.json') ? 'json' : '' }} /></button>)}
        </div>
      </div> : <div className={classes("contact-picker")}>{contacts.map((contact) => <button type="button" key={contact.userId} onClick={() => void sendContact(contact.userId)}><Avatar name={contact.fullName} url={contact.avatarUrl} size={36} /><span><strong>{contact.fullName}</strong><small>{contact.identityNumber}</small></span></button>)}</div>}</div>}
      {quote && (
        <div className={classes("composer-quote")} style={{ '--quote-color': quoteColor } as CSSProperties}>
          <span><strong>{quote.senderName}</strong><small>{quote.content || `[${quote.category}]`}</small></span>
          {quote.category.includes('CONTACT') ? <Avatar name={quote.sharedUserFullName || 'Contact'} url={quote.sharedUserAvatarUrl} size={48} /> : quoteSource && <img src={quoteSource} alt="" />}
          <IconButton label="Cancel reply" type="button" onClick={() => setQuote()}><X size={16} /></IconButton>
        </div>
      )}
      <div className={classes("composer-actions")}>
        <CustomPopupMenuButton
          side="top"
          trigger={<span><IconButton label="Add attachment" type="button" disabled={attachmentPending}><Plus size={22} /></IconButton></span>}
          onSelected={(action) => { if (action === 'contact') void openContacts(); else void chooseAttachment(action === 'media'); }}
          items={[
            { value: 'contact', title: 'Contact', icon: <Paperclip size={16} /> },
            { value: 'files', title: 'Files', icon: <FileText size={16} /> },
            { value: 'media', title: 'Pictures and Videos', icon: <ImageIcon size={16} /> },
          ]}
        />
        <IconButton label="Stickers" type="button" onClick={() => void openStickers()}><Smile size={20} /></IconButton>
      </div>
      <textarea
        ref={textareaRef}
        value={value}
        rows={1}
        placeholder={encryption.data ? 'End-to-end encrypted' : 'Type message'}
        onChange={(event) => {
          updateValue(event.target.value);
          updateMentionKeyword(event.target.value, event.target.selectionStart);
        }}
        onSelect={(event) => updateMentionKeyword(event.currentTarget.value, event.currentTarget.selectionStart)}
        onContextMenu={(event) => { if (!event.defaultPrevented) selectionToolbar.onContextMenu(event); }}
        onKeyDown={onKeyDown}
      />
      {selectionToolbar.toolbar}
      {recording ? <div className={classes("voice-recorder")}><IconButton label="Cancel recording" type="button" onClick={() => void cancelRecording()}><X size={20} /></IconButton>{recording === 'recording' ? <span className={classes("voice-recording-duration")}><i />{Math.floor(recordingSeconds / 60)}:{String(recordingSeconds % 60).padStart(2, '0')}</span> : <span className={classes("voice-recording-preview")}><IconButton label={previewPlaying ? 'Stop preview' : 'Play preview'} type="button" onClick={() => { const audio = previewAudioRef.current; if (!audio) return; if (audio.paused) void audio.play(); else audio.pause(); }}>{previewPlaying ? <Pause size={18} /> : <Play size={18} />}</IconButton><span className={classes("voice-waveform")}>{recording.waveform.map((value, index) => <i key={index} style={{ height: Math.max(2, Math.round(value / 255 * 20)) }} />)}</span><small>{formatDuration(String(recording.durationMillis))}</small><audio ref={previewAudioRef} src={resolveAssetUrl(recording.path)} onPlay={() => setPreviewPlaying(true)} onPause={() => setPreviewPlaying(false)} onEnded={() => setPreviewPlaying(false)} /></span>}{recording === 'recording' ? <IconButton label="Stop recording" type="button" className={classes("voice-stop")} onClick={() => void stopRecording()}><X size={18} /></IconButton> : <IconButton label="Retry recording" type="button" onClick={() => void retryRecording()}><RotateCw size={18} /></IconButton>}<IconButton label="Send voice message" type="button" onClick={() => void sendRecording()}><Send size={20} /></IconButton></div> : value.trim() ? <span className={classes("composer-send-action")} onContextMenu={(event) => void showNativeContextMenu(event, [{ type: 'item', id: 'send-without-sound', text: 'Send without sound', icon: flutterMenuIcon('mute'), action: () => send.mutate({ content: value.trim(), silent: true }) }])}><IconButton label="Send message" className={classes("send-button")} type="submit" disabled={send.isPending}><Send size={19} /></IconButton></span> : <IconButton label="Record voice message" type="button" onClick={() => void startRecording()}><Mic size={20} /></IconButton>}
      {send.isError && <div className={classes("composer-error")}>Message failed to send</div>}
      {attachmentError && <div className={classes("composer-error")}>{attachmentError}</div>}
      {attachmentDrafts && <AttachmentPreviewDialog drafts={attachmentDrafts} sending={attachmentPending} onClose={() => setAttachmentDrafts(undefined)} onSend={(drafts, mode, caption, password, silent) => void sendAttachments(drafts, mode, caption, password, silent)} />}
      {stickerDraftPath && <AddStickerDialog path={stickerDraftPath} onClose={() => setStickerDraftPath(undefined)} onSave={() => void saveSticker()} />}
      {storeOpen && <StickerStoreDialog albums={storeAlbums} stickers={storeAlbumStickers} onClose={() => setStoreOpen(false)} onToggle={toggleStoreAlbum} onAlbumsChanged={async () => setStickerAlbums(await desktopApi.stickerAlbums())} />}
    </form>
  );
}

function AttachmentPreviewDialog({ drafts: initialDrafts, sending, onClose, onSend }: { drafts: AttachmentDraft[]; sending: boolean; onClose: () => void; onSend: (drafts: AttachmentDraft[], mode: AttachmentPreviewMode, caption: string, password: string, silent: boolean) => void }) {
  const [drafts, setDrafts] = useState(initialDrafts);
  const [mode, setMode] = useState<AttachmentPreviewMode>(() => initialDrafts.some((draft) => draft.kind !== 'DATA') ? 'media' : 'files');
  const [caption, setCaption] = useState('');
  const [password, setPassword] = useState('');
  const [passwordVisible, setPasswordVisible] = useState(false);
  const [editingIndex, setEditingIndex] = useState<number>();
  const hasMedia = drafts.some((draft) => draft.kind !== 'DATA');
  const singleImage = drafts.length === 1 && drafts[0]?.kind === 'IMAGE';
  const activeMode = attachmentPreviewMode(drafts, mode);
  const remove = (path: string) => setDrafts((current) => {
    const next = current.filter((draft) => draft.path !== path);
    if (next.length === 0) onClose();
    return next;
  });
  const addFiles = async () => {
    const selected = await open({ multiple: true, directory: false });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    const known = new Set(drafts.map((draft) => draft.path));
    const infos = await desktopApi.attachmentFileInfo(paths.filter((path) => !known.has(path)));
    setDrafts((current) => [...current, ...infos.map(attachmentDraft)]);
  };
  const edit = async (dataUrl: string) => {
    if (editingIndex === undefined) return;
    const path = await desktopApi.writeEditedImage(dataUrl);
    const [info] = await desktopApi.attachmentFileInfo([path]);
    setDrafts((current) => current.map((draft, index) => index === editingIndex ? attachmentDraft(info) : draft));
    setEditingIndex(undefined);
  };
  const send = (silent: boolean) => onSend(drafts, activeMode, caption, password, silent);
  return <>{editingIndex !== undefined && drafts[editingIndex] && <ImageEditorDialog path={drafts[editingIndex].path} onClose={() => setEditingIndex(undefined)} onDone={(dataUrl) => void edit(dataUrl)} />}<div className={classes("modal-backdrop attachment-preview-backdrop")} role="presentation" onMouseDown={() => !sending && onClose()}><section className={classes("attachment-preview-dialog")} role="dialog" aria-modal="true" aria-label="Attachment preview" onMouseDown={(event) => event.stopPropagation()} onKeyDown={(event) => { if (event.key === 'Enter' && !event.shiftKey && !sending) { event.preventDefault(); send(false); } }}><div className={classes("attachment-preview-tabs")}><button type="button" title="Send quickly" className={classes(activeMode === 'media' ? 'is-selected' : '')} disabled={!hasMedia} onClick={() => setMode('media')}><ImageIcon size={24} /></button><button type="button" title="Send without compression" className={classes(activeMode === 'files' ? 'is-selected' : '')} onClick={() => setMode('files')}><FileText size={24} /></button>{drafts.length > 1 && <button type="button" title="Send as ZIP" className={classes(activeMode === 'zip' ? 'is-selected' : '')} onClick={() => setMode('zip')}><span>ZIP</span></button>}<button type="button" title="Add files" className={classes("attachment-preview-add")} onClick={() => void addFiles()}><Plus size={20} /></button></div><div className={classes("attachment-preview-content")}>{activeMode === 'zip' ? <div className={classes("attachment-preview-zip")}><div className={classes("attachment-file-icon")}>ZIP</div><div><strong>Archive.zip</strong><small>Archived Folder</small></div><label><Lock size={18} /><input autoFocus type={passwordVisible ? 'text' : 'password'} value={password} maxLength={128} placeholder="Password" onChange={(event) => setPassword(event.target.value)} /><button type="button" aria-label={passwordVisible ? 'Hide password' : 'Show password'} onClick={() => setPasswordVisible((visible) => !visible)}>{passwordVisible ? <EyeOff size={18} /> : <Eye size={18} />}</button></label></div> : <div className={classes("attachment-preview-list")}>{drafts.map((draft, index) => { const source = resolveAssetUrl(draft.path); const actions = <div className={classes("attachment-preview-media-actions")}>{draft.kind === 'IMAGE' && <button type="button" aria-label="Edit attachment" onClick={() => setEditingIndex(index)}><Pencil size={24} /></button>}<button type="button" aria-label="Delete attachment" onClick={() => remove(draft.path)}><Trash2 size={24} /></button></div>; return <article key={draft.path} className={classes(activeMode === 'media' && draft.kind !== 'DATA' ? 'attachment-preview-media' : 'attachment-preview-file')}>{activeMode === 'media' && draft.kind === 'IMAGE' && source && <><img src={source} alt="" />{actions}</>}{activeMode === 'media' && draft.kind === 'VIDEO' && source && <><video src={source} autoPlay muted loop />{actions}</>}{(activeMode === 'files' || draft.kind === 'DATA') && <><div className={classes("attachment-file-icon")}>{draft.name.split('.').pop()?.toUpperCase().slice(0, 5) || 'FILE'}</div><div><strong>{draft.name}</strong><small>{formatAttachmentBytes(draft.size)}</small></div><button type="button" aria-label="Delete attachment" onClick={() => remove(draft.path)}><Trash2 size={24} /></button></>}</article>; })}</div>}</div>{singleImage && activeMode !== 'zip' && <textarea className={classes("attachment-caption")} autoFocus value={caption} maxLength={64 * 1024} placeholder="Add a caption" onChange={(event) => setCaption(event.target.value)} />}<span className={classes("attachment-send-action")} onContextMenu={(event) => void showNativeContextMenu(event, [{ type: 'item', id: 'send-attachment-without-sound', text: 'Send without sound', icon: flutterMenuIcon('mute'), action: () => send(true) }])}><button type="button" disabled={sending || drafts.length === 0} onClick={() => send(false)}>SEND</button></span><small className={classes("attachment-enter-hint")}>Enter to send</small><IconButton className={classes("attachment-preview-close")} type="button" label="Cancel attachment" onClick={onClose}><X size={20} /></IconButton></section></div></>;
}

function AddStickerDialog({ path, onClose, onSave }: { path: string; onClose: () => void; onSave: () => void }) {
  const source = resolveAssetUrl(path);
  return <div className={classes("modal-backdrop add-sticker-backdrop")} role="presentation" onMouseDown={onClose}><section className={classes("add-sticker-dialog")} role="dialog" aria-modal="true" aria-label="Add Sticker" onMouseDown={(event) => event.stopPropagation()}><header><span /><strong>Add Sticker</strong><IconButton label="Close add sticker" onClick={onClose}><X size={20} /></IconButton></header><div>{source && <div className={classes("add-sticker-preview")}><img src={source} alt="Sticker preview" /></div>}</div><footer><button type="button" onClick={onSave}>Save</button></footer></section></div>;
}

async function attachmentMetadata(path: string, kind: string): Promise<{ width: number; height: number; durationMillis?: number } | undefined> {
  if (kind === 'DATA') return undefined;
  const source = resolveAssetUrl(path);
  if (!source) return undefined;
  if (kind === 'IMAGE') return new Promise((resolve) => {
    const image = new window.Image();
    image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight });
    image.onerror = () => resolve(undefined);
    image.src = source;
  });
  return new Promise((resolve) => {
    const video = document.createElement('video');
    video.preload = 'metadata';
    video.onloadedmetadata = () => resolve({ width: video.videoWidth, height: video.videoHeight, durationMillis: Math.round(video.duration * 1000) });
    video.onerror = () => resolve(undefined);
    video.src = source;
  });
}
