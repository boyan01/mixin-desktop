import { useEffect, useState, type MouseEvent } from 'react';
import { Info, MessageCircle, Share2, X } from 'lucide-react';
import { desktopApi } from '@/shared/api/desktopApi';
import { e } from '@/shared/lib/logger';
import type { UserProfile } from '@/shared/model/types';
import { Avatar } from '@/shared/ui/Avatar';
import { Badges } from '@/shared/ui/Badges';
import { MoreExtendedText } from '@/shared/ui/RichText';
import { classes } from '@/shared/styles/classes';

export function UserProfileDialog({ profile, currentUserId, onClose, onChanged, onChat, onInfo }: { profile: UserProfile; currentUserId: string; onClose: () => void; onChanged: () => Promise<void>; onChat: () => Promise<void>; onInfo: () => Promise<void> }) {
  const [currentProfile, setCurrentProfile] = useState(profile);
  const [saving, setSaving] = useState(false);
  const anonymous = currentProfile.identityNumber === '0';
  useEffect(() => { setCurrentProfile(profile); }, [profile]);
  useEffect(() => {
    let disposed = false;
    void desktopApi.refreshUserProfile(profile.userId).then((refreshed) => {
      if (!disposed && refreshed) setCurrentProfile(refreshed);
    }).catch((reason) => e('Unable to refresh user profile', reason));
    return () => { disposed = true; };
  }, [profile.userId]);
  async function updateRelationship() {
    if (saving) return;
    setSaving(true);
    try {
      if (currentProfile.relationship === 'FRIEND') await desktopApi.removeContact(currentProfile.userId);
      else if (currentProfile.relationship === 'BLOCKED') await desktopApi.unblockUser(currentProfile.userId);
      else await desktopApi.addContact(currentProfile.userId, currentProfile.fullName);
      await onChanged();
    } finally { setSaving(false); }
  }
  function copyUriOnAlt(event: MouseEvent<HTMLSpanElement>) {
    if (event.altKey) void navigator.clipboard.writeText(`mixin://users/${currentProfile.userId}`);
  }
  return <div className={classes("modal-backdrop")}><section className={classes("user-profile-dialog")} role="dialog" aria-modal="true" aria-label={currentProfile.fullName}><button type="button" className={classes("user-profile-close")} aria-label="Close profile" onClick={onClose}><X size={20} /></button><span className={classes("user-profile-avatar")} onClick={copyUriOnAlt}><Avatar name={currentProfile.fullName} url={currentProfile.avatarUrl} size={90} /></span><h2>{currentProfile.fullName}<Badges verified={currentProfile.isVerified} isBot={currentProfile.isBot} membership={currentProfile.membership} /></h2>{!anonymous && <small>Mixin ID: {currentProfile.identityNumber}</small>}{!anonymous && currentProfile.relationship.toUpperCase() === 'STRANGER' && <button type="button" className={classes("add-contact")} disabled={saving} onClick={() => void updateRelationship()}>+ {currentProfile.isBot ? 'Add Bot' : 'Add Contact'}</button>}{currentProfile.biography.trim() && <MoreExtendedText text={currentProfile.biography} className={classes("user-profile-biography")} />}{!anonymous && <footer><button type="button" aria-label="Share contact" onClick={() => void navigator.clipboard.writeText(currentProfile.codeUrl)}><Share2 size={30} /></button>{currentProfile.userId !== currentUserId && <button type="button" aria-label="Chat" onClick={() => void onChat()}><MessageCircle size={30} /></button>}{currentProfile.userId !== currentUserId && <button type="button" aria-label="Contact info" onClick={() => void onInfo()}><Info size={30} /></button>}</footer>}</section></div>;
}
