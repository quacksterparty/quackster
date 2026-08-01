import { describe, it, expect, vi } from 'vitest';

vi.mock('$lib/paraglide/messages', () => ({
	m: { game_answer_true: () => 'True', game_answer_false: () => 'False' }
}));

import type { AnswerView } from '$lib/bindings/Protocol';
import { correctnessText } from './correctness';

function answer(correctness: AnswerView['correctness']): AnswerView {
	return {
		locale: 'en',
		correctness,
		canonical_locale: null,
		canonical_correctness: null,
		explanation: null
	};
}

describe('correctnessText', () => {
	it('MultipleChoice resolves correct ids to choice text', () => {
		expect(
			correctnessText(answer({ kind: 'MultipleChoice', correct_ids: ['b', 'c'] }), {
				kind: 'MultipleChoice',
				choices: [
					{ id: 'a', text: 'A', media: null },
					{ id: 'b', text: 'B', media: null },
					{ id: 'c', text: 'C', media: null }
				]
			})
		).toBe('B, C');
	});

	it('Open joins accepted answers', () => {
		expect(
			correctnessText(answer({ kind: 'Open', accepted: ['Paris', 'Lutetia'] }), { kind: 'Open' })
		).toBe('Paris, Lutetia');
	});

	it('TrueFalse maps the boolean', () => {
		expect(
			correctnessText(answer({ kind: 'TrueFalse', correct: true }), { kind: 'TrueFalse' })
		).toBe('True');
	});

	it('Numeric shows tolerance only when positive', () => {
		expect(
			correctnessText(answer({ kind: 'Numeric', value: 42, tolerance: 0 }), {
				kind: 'NumericInput'
			})
		).toBe('42');
		expect(
			correctnessText(answer({ kind: 'Numeric', value: 42, tolerance: 2 }), {
				kind: 'NumericInput'
			})
		).toBe('42 ± 2');
	});

	it('Order sorts by position and resolves item text', () => {
		expect(
			correctnessText(
				answer({
					kind: 'Order',
					positions: [
						{ id: 'y', position: 1 },
						{ id: 'x', position: 0 }
					]
				}),
				{
					kind: 'Order',
					items: [
						{ id: 'x', text: 'First', media: null },
						{ id: 'y', text: 'Second', media: null }
					]
				}
			)
		).toBe('First → Second');
	});
});
