import botBadgeUrl from '@flutter-assets/images/bot_fill.svg';
import basicPlanUrl from '@flutter-assets/images/plan_basic.png';
import standardPlanUrl from '@flutter-assets/images/plan_standard.png';
import premiumPlanUrl from '@flutter-assets/images/plan_premium.gif';
import verifiedUrl from '@flutter-assets/images/verified.svg';
import { classes } from '@/shared/styles/classes';

function validMembershipPlan(raw?: string | null): 'advance' | 'elite' | 'prosperity' | undefined {
  if (!raw) return undefined;
  try {
    const membership = JSON.parse(raw) as { plan?: unknown; expired_at?: unknown };
    const expiresAt = typeof membership.expired_at === 'string' ? Date.parse(membership.expired_at) : Number.NaN;
    return Number.isFinite(expiresAt) && expiresAt > Date.now() && (membership.plan === 'advance' || membership.plan === 'elite' || membership.plan === 'prosperity')
      ? membership.plan
      : undefined;
  } catch { return undefined; }
}

export function Badges({ verified, isBot, membership }: { verified: boolean; isBot: boolean; membership?: string | null }) {
  const plan = validMembershipPlan(membership);
  const asset = plan === 'advance' ? basicPlanUrl : plan === 'elite' ? standardPlanUrl : plan === 'prosperity' ? premiumPlanUrl : verified ? verifiedUrl : isBot ? botBadgeUrl : undefined;
  const size = plan ? 14 : 12;
  return asset ? <img className={classes("badges")} src={asset} width={size} height={size} alt="" /> : null;
}
