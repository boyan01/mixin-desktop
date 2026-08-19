import chat from './chat.module.css';
import composer from './composer.module.css';
import conversationDetails from './conversation-details.module.css';
import conversations from './conversations.module.css';
import dialogs from './dialogs.module.css';
import protocolDialogs from './protocol-dialogs.module.css';
import responsive from './responsive.module.css';
import settings from './settings.module.css';
import sharedControls from './shared-controls.module.css';
import shell from './shell.module.css';
import sidebar from './sidebar.module.css';
import startup from './startup.module.css';

type ClassValue = string | false | null | undefined;

const globalClassNames = new Set(['theme-light', 'theme-dark', 'theme-system']);
const modules = [
  chat,
  composer,
  conversationDetails,
  conversations,
  dialogs,
  protocolDialogs,
  responsive,
  settings,
  sharedControls,
  shell,
  sidebar,
  startup,
];

const classNamesByLocalName = modules.reduce<Record<string, string[]>>((result, module) => {
  for (const [localName, className] of Object.entries(module)) {
    const names = result[localName] ?? (result[localName] = []);
    if (!names.includes(className)) names.push(className);
  }
  return result;
}, {});

function resolveClassName(localName: string): string {
  const moduleClassNames = classNamesByLocalName[localName] ?? [];
  return globalClassNames.has(localName)
    ? [localName, ...moduleClassNames].join(' ')
    : moduleClassNames.join(' ') || localName;
}

export function classes(...values: ClassValue[]): string {
  return values
    .filter((value): value is string => typeof value === 'string' && value.length > 0)
    .flatMap((value) => value.trim().split(/\s+/).map(resolveClassName))
    .join(' ');
}
