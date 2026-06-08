import { TestBed } from '@angular/core/testing';
import { LoggerService } from './logger.service';

describe('LoggerService', () => {
  let service: LoggerService;
  let emitted: unknown[];

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(LoggerService);
    emitted = [];
    service.sink = (entry) => emitted.push(entry);
  });

  it('emits a structured entry with level, message and context', () => {
    service.info('user.login', { method: 'passkey' });
    expect(emitted.length).toBe(1);
    const entry = emitted[0] as Record<string, unknown>;
    expect(entry['level']).toBe('info');
    expect(entry['message']).toBe('user.login');
    expect(entry['context']).toEqual({ method: 'passkey' });
    expect(typeof entry['timestamp']).toBe('string');
  });

  it('supports warn and error levels', () => {
    service.warn('job.retry', {});
    service.error('job.failed', { attempts: 3 });
    const levels = emitted.map((e) => (e as Record<string, unknown>)['level']);
    expect(levels).toEqual(['warn', 'error']);
  });
});
