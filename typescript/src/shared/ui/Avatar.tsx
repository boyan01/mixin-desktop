import { resolveAssetUrl } from '@/shared/api/desktopApi';
import { classes } from '@/shared/styles/classes';
import { MixinImage } from '@/shared/ui/MixinImage';

interface AvatarProps {
  name: string;
  url?: string | null;
  size?: number;
  className?: string;
}

const gradients = [
  ['#82c7ff', '#3d75e3'],
  ['#8fd8a0', '#45a75b'],
  ['#d5a6ff', '#8057c8'],
  ['#ffbd8a', '#e47a4d'],
  ['#8bd8d1', '#418f99'],
];

function gradientFor(value: string): string {
  const hash = [...value].reduce((result, character) => result + character.charCodeAt(0), 0);
  const [start, end] = gradients[hash % gradients.length];
  return `linear-gradient(145deg, ${start}, ${end})`;
}

export function Avatar({ name, url, size = 42, className = '' }: AvatarProps) {
  const resolved = resolveAssetUrl(url);
  const initial = name.trim().slice(0, 1).toUpperCase() || '?';
  return (
    <span
      className={classes(`avatar ${className}`)}
      style={{ width: size, height: size, background: gradientFor(name) }}
      aria-label={name}
    >
      {resolved ? <MixinImage source={resolved} alt="" width={size} height={size} fit="cover" placeholder={<span className={classes("avatar-placeholder")}>{initial}</span>} /> : initial}
    </span>
  );
}
