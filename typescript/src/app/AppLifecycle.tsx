import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useRef, type ReactNode } from 'react';

interface AppLifecycleProps {
  children: ReactNode;
}

export function AppLifecycle({ children }: AppLifecycleProps) {
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    let previousEditableFocus: HTMLInputElement | HTMLTextAreaElement | null = null;
    const collapseSelection = (element: HTMLInputElement | HTMLTextAreaElement | null) => {
      if (!element || element.selectionStart === null) return;
      element.setSelectionRange(element.selectionStart, element.selectionStart);
    };
    const onFocusIn = (event: FocusEvent) => {
      const target = event.target;
      const next = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement ? target : null;
      if (next === previousEditableFocus) return;
      collapseSelection(previousEditableFocus);
      previousEditableFocus = next;
    };
    const onVisibilityChange = () => {
      if (document.hidden) {
        const active = document.activeElement;
        previousFocus.current = active instanceof HTMLElement ? active : null;
        previousFocus.current?.blur();
        return;
      }
      previousFocus.current?.focus();
      previousFocus.current = null;
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'w') {
        event.preventDefault();
        void getCurrentWindow().hide();
      }
    };

    document.addEventListener('focusin', onFocusIn);
    document.addEventListener('visibilitychange', onVisibilityChange);
    window.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('focusin', onFocusIn);
      document.removeEventListener('visibilitychange', onVisibilityChange);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  return children;
}
