import * as Tooltip from '@radix-ui/react-tooltip';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { DocumentSelectionToolbar } from '@/shared/ui/AdaptiveSelectionToolbar';
import { AppLifecycle } from './AppLifecycle';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10_000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <Tooltip.Provider delayDuration={500}>
        <AppLifecycle>{children}</AppLifecycle>
        <DocumentSelectionToolbar />
      </Tooltip.Provider>
    </QueryClientProvider>
  );
}
