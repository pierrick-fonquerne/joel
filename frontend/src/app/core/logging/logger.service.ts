import { Injectable } from '@angular/core';

export type LogLevel = 'info' | 'warn' | 'error';

export interface LogEntry {
  level: LogLevel;
  message: string;
  context: Record<string, unknown>;
  timestamp: string;
}

/**
 * Structured JSON logger. Console transport lives here and only here:
 * application code must never call console directly.
 */
@Injectable({ providedIn: 'root' })
export class LoggerService {
  sink: (entry: LogEntry) => void = (entry) => {
    // eslint-disable-next-line no-console
    console[entry.level](JSON.stringify(entry));
  };

  info(message: string, context: Record<string, unknown> = {}): void {
    this.emit('info', message, context);
  }

  warn(message: string, context: Record<string, unknown> = {}): void {
    this.emit('warn', message, context);
  }

  error(message: string, context: Record<string, unknown> = {}): void {
    this.emit('error', message, context);
  }

  private emit(level: LogLevel, message: string, context: Record<string, unknown>): void {
    this.sink({ level, message, context, timestamp: new Date().toISOString() });
  }
}
