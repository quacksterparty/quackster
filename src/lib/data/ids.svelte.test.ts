import { describe, it, expect } from 'vitest';
import { isDraftId, namedQuestionId, slugify } from './ids';

describe('slugify', () => {
	it('lowercases and replaces non-alphanumerics with underscore', () => {
		expect(slugify('Hello, World!')).toBe('hello_world');
	});

	it('strips diacritics', () => {
		expect(slugify('Über die Größe')).toBe('uber_die_grosse');
	});

	it('trims leading and trailing underscores', () => {
		expect(slugify('---foo---')).toBe('foo');
	});

	it('caps length at 48 chars', () => {
		const long = 'a'.repeat(60);
		expect(slugify(long).length).toBeLessThanOrEqual(48);
	});

	it('returns empty for input with no slug-friendly chars', () => {
		expect(slugify('!!!')).toBe('');
	});
});

describe('namedQuestionId', () => {
	it('uses base id when no collision', () => {
		expect(namedQuestionId('foo', new Set())).toBe('q_foo');
	});

	it('appends random suffix on collision', () => {
		const taken = new Set(['q_foo']);
		const id = namedQuestionId('foo', taken);
		expect(id.startsWith('q_foo_')).toBe(true);
		expect(id).not.toBe('q_foo');
		expect(taken.has(id)).toBe(false);
	});

	it('falls back to base id for empty slug', () => {
		expect(namedQuestionId('', new Set())).toBe('q_unnamed');
	});

	it('produces different suffixes across calls when collision persists', () => {
		const taken = new Set(['q_foo', 'q_foo_a']);
		const id = namedQuestionId('foo', taken);
		expect(id.startsWith('q_foo_')).toBe(true);
		expect(taken.has(id)).toBe(false);
	});
});

describe('isDraftId', () => {
	it('matches the q_draft_ prefix', () => {
		expect(isDraftId('q_draft_abc123')).toBe(true);
	});

	it('rejects named ids', () => {
		expect(isDraftId('q_foo')).toBe(false);
	});

	it('rejects empty string', () => {
		expect(isDraftId('')).toBe(false);
	});
});
