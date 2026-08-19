import { create } from 'zustand';
import type { ConversationCategory, Message, UiSettings } from '@/shared/model/types';

interface DesktopUiState {
  selectedConversationId?: string;
  category: ConversationCategory;
  circleId?: string;
  query: string;
  unseenOnly: boolean;
  sidebarCollapsed: boolean;
  infoOpen: boolean;
  quote?: Message;
  messageSearchOpen: boolean;
  messageSearchFromInfo: boolean;
  focusMessageId?: string;
  settingsOpen: boolean;
  deviceTransferOpen: boolean;
  createMode?: 'search-contact' | 'conversation' | 'group' | 'circle';
  menuConversationAction?: 'mute' | 'delete';
  forwardMessageIds: string[];
  forwardCombined: boolean;
  selectionConversationId?: string;
  selectedMessageIds: string[];
  drafts: Record<string, string>;
  uiSettings: UiSettings;
  selectConversation: (conversationId?: string) => void;
  setCategory: (category: ConversationCategory, circleId?: string) => void;
  setQuery: (query: string) => void;
  toggleUnseenOnly: () => void;
  toggleSidebar: () => void;
  toggleInfo: () => void;
  closeInfo: () => void;
  setQuote: (quote?: Message) => void;
  openMessageSearch: () => void;
  closeMessageSearch: (restoreInfo?: boolean) => void;
  focusMessage: (messageId: string) => void;
  clearFocusMessage: (messageId?: string) => void;
  openSettings: () => void;
  closeSettings: () => void;
  openDeviceTransfer: () => void;
  closeDeviceTransfer: () => void;
  openCreate: (mode: 'search-contact' | 'conversation' | 'group' | 'circle') => void;
  closeCreate: () => void;
  requestMenuConversationAction: (action: 'mute' | 'delete') => void;
  clearMenuConversationAction: () => void;
  openForward: (messageIds: string[], combined?: boolean) => void;
  closeForward: () => void;
  beginMessageSelection: (conversationId: string, messageId: string) => void;
  toggleMessageSelection: (messageId: string) => void;
  clearMessageSelection: () => void;
  setDraft: (conversationId: string, draft: string) => void;
  setUiSettings: (settings: Partial<UiSettings>) => void;
}

export const useDesktopUi = create<DesktopUiState>((set) => ({
  category: 'chats',
  query: '',
  unseenOnly: false,
  sidebarCollapsed: false,
  infoOpen: false,
  messageSearchOpen: false,
  messageSearchFromInfo: false,
  settingsOpen: false,
  deviceTransferOpen: false,
  forwardMessageIds: [],
  forwardCombined: false,
  selectedMessageIds: [],
  drafts: {},
  uiSettings: {
    theme: 'system',
    showAvatar: true,
    showIdentityNumber: false,
    messagePreview: true,
    chatFontSizeDelta: 0,
  },
  selectConversation: (selectedConversationId) => set({ selectedConversationId, infoOpen: false, messageSearchOpen: false, messageSearchFromInfo: false, focusMessageId: undefined, quote: undefined }),
  setCategory: (category, circleId) => set({ category, circleId, selectedConversationId: undefined, infoOpen: false, settingsOpen: false }),
  setQuery: (query) => set({ query }),
  toggleUnseenOnly: () => set((state) => ({ unseenOnly: !state.unseenOnly })),
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  toggleInfo: () => set((state) => ({ infoOpen: !state.infoOpen })),
  closeInfo: () => set({ infoOpen: false }),
  setQuote: (quote) => set({ quote }),
  openMessageSearch: () => set((state) => ({ messageSearchOpen: true, messageSearchFromInfo: state.infoOpen, infoOpen: false })),
  closeMessageSearch: (restoreInfo = true) => set((state) => ({ messageSearchOpen: false, infoOpen: restoreInfo && state.messageSearchFromInfo, messageSearchFromInfo: false })),
  focusMessage: (focusMessageId) => set({ focusMessageId }),
  // Do not let a delayed highlight clear a newer locate-to-message request.
  clearFocusMessage: (messageId) => set((state) =>
    messageId === undefined || state.focusMessageId === messageId
      ? { focusMessageId: undefined }
      : state,
  ),
  openSettings: () => set({ settingsOpen: true }),
  closeSettings: () => set({ settingsOpen: false }),
  openDeviceTransfer: () => set({ deviceTransferOpen: true }),
  closeDeviceTransfer: () => set({ deviceTransferOpen: false }),
  openCreate: (createMode) => set({ createMode }),
  closeCreate: () => set({ createMode: undefined }),
  requestMenuConversationAction: (menuConversationAction) => set({ menuConversationAction }),
  clearMenuConversationAction: () => set({ menuConversationAction: undefined }),
  openForward: (forwardMessageIds, forwardCombined = false) => set({ forwardMessageIds, forwardCombined }),
  closeForward: () => set({ forwardMessageIds: [], forwardCombined: false }),
  beginMessageSelection: (selectionConversationId, messageId) => set({ selectionConversationId, selectedMessageIds: [messageId] }),
  toggleMessageSelection: (messageId) => set((state) => ({ selectedMessageIds: state.selectedMessageIds.includes(messageId) ? state.selectedMessageIds.filter((id) => id !== messageId) : [...state.selectedMessageIds, messageId] })),
  clearMessageSelection: () => set({ selectionConversationId: undefined, selectedMessageIds: [] }),
  setDraft: (conversationId, draft) =>
    set((state) => ({ drafts: { ...state.drafts, [conversationId]: draft } })),
  setUiSettings: (settings) => set((state) => ({ uiSettings: { ...state.uiSettings, ...settings } })),
}));
