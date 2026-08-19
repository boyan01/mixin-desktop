import { desktopApi } from '@/shared/api/desktopApi';

type LogLevel = 'verbose' | 'debug' | 'info' | 'warning' | 'error' | 'wtf';

export function v(message: string): void {
  write('verbose', message);
}

export function d(message: string): void {
  write('debug', message);
}

export function i(message: string): void {
  write('info', message);
}

export function w(message: string): void {
  write('warning', message);
}

export function e(message: string, error?: unknown, stackTrace?: string): void {
  const details = error === undefined ? message : `${message} (${formatError(error)})`;
  const stack = stackTrace ?? (error instanceof Error ? error.stack : undefined);
  write('error', stack ? `${details}:\n${stack}` : details);
}

export function wtf(message: string): void {
  write('wtf', message);
}

function write(level: LogLevel, message: string): void {
  void desktopApi.logTypescript(level, message).catch(() => undefined);
}

function formatError(error: unknown): string {
  if (error instanceof Error) return `${error.name}: ${error.message}`;
  if (typeof error === 'object' && error !== null && 'message' in error) {
    return `${error.constructor.name}(${String(error.message)})`;
  }
  return String(error);
}
