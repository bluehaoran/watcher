import { describe, it, expect } from 'vitest';

describe('Simple Test', () => {
  it('should verify test infrastructure is working', () => {
    expect(1 + 1).toBe(2);
  });

  it('should test environment variables', () => {
    expect(process.env.NODE_ENV).toBe('test');
    expect(process.env.SECRET_KEY).toBe('test-secret-key-that-is-at-least-32-characters-long');
  });
});