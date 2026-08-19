import type { MacosMenuLabels, MacosMenuState } from '@/generated/bindings';
import type { Conversation } from '@/shared/model/types';

export type { MacosMenuLabels, MacosMenuState } from '@/generated/bindings';

// These names intentionally match flutter/lib/l10n/intl_en.arb. The bridge
// accepts this frontend-owned map so UI localization can supply its active copy.
export const englishMacosMenuLabels: MacosMenuLabels = {
  about: 'About',
  preferences: 'Preferences',
  lock: 'Lock',
  quickSearch: 'Quick Search',
  hideMixin: 'Hide Mixin',
  showMixin: 'Show Mixin',
  quitMixin: 'Quit Mixin',
  file: 'File',
  edit: 'Edit',
  createConversation: 'New Conversation',
  createGroup: 'New Group',
  createCircle: 'New Circle',
  closeWindow: 'Close window',
  conversation: 'Conversation',
  mute: 'Mute',
  unmute: 'Unmute',
  search: 'Search',
  deleteChat: 'Delete Chat',
  pinTitle: 'Pin',
  unpin: 'Unpin',
  toggleChatInfo: 'Toggle chat info',
  window: 'Window',
  minimize: 'Minimize',
  zoom: 'Zoom',
  previousConversation: 'Previous conversation',
  nextConversation: 'Next conversation',
  bringAllToFront: 'Bring All to Front',
  help: 'Help',
  helpCenter: 'Help Center',
  termsOfService: 'Terms of Service',
  privacyPolicy: 'Privacy Policy',
};

/** Flutter `intl_zh.arb` values for the native menu keys. */
export const chineseMacosMenuLabels: MacosMenuLabels = {
  about: '关于', preferences: '偏好设置', lock: '锁定', quickSearch: '快速搜索',
  hideMixin: '隐藏 Mixin', showMixin: '显示 Mixin', quitMixin: '退出 Mixin', file: '文件',
  createConversation: '新建会话', createGroup: '新建群组', createCircle: '新建圈子', closeWindow: '关闭窗口',
  conversation: '会话', mute: '静音', unmute: '取消静音', search: '搜索', deleteChat: '删除聊天',
  pinTitle: '置顶', unpin: '取消置顶', toggleChatInfo: '展开/关闭会话信息', window: '窗口', edit: '编辑',
  minimize: '最小化', zoom: '缩放', previousConversation: '上一个会话', nextConversation: '下一个会话',
  bringAllToFront: '前置所有窗口', help: '帮助', helpCenter: '帮助中心', termsOfService: '服务条款', privacyPolicy: '隐私政策',
};

/** Maps Flutter's currently bundled Chinese and English desktop menu catalogs. */
export function macosMenuLabels(locale: string | undefined = navigator.language): MacosMenuLabels {
  return locale.toLowerCase().startsWith('zh') ? chineseMacosMenuLabels : englishMacosMenuLabels;
}

export function macosMenuState(
  conversation: Conversation | undefined,
  hasPasscode: boolean,
  labels: MacosMenuLabels = macosMenuLabels(),
): MacosMenuState {
  return {
    labels,
    hasConversation: conversation !== undefined,
    isMuted: conversation?.isMuted ?? false,
    isPinned: conversation?.isPinned ?? false,
    hasPasscode,
  };
}
