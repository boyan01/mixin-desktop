import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from '@/app/App';
import { AppProviders } from '@/app/AppProviders';
import '@/shared/debug/hmrMonitor';
import { i } from '@/shared/lib/logger';
import { installScrollbarInteraction } from '@/shared/lib/scrollbar';
import '@/shared/styles/global.css';

i('Application started');
document.addEventListener('contextmenu', (event) => event.preventDefault());
installScrollbarInteraction();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AppProviders>
      <App />
    </AppProviders>
  </StrictMode>,
);
