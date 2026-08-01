<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { isDraftId } from '$lib/data/ids';
	import Button from '../Button.svelte';

	let {
		activeDraftId,
		activeCell,
		activeQuestionId,
		onPickQuestion,
		onCreateNew,
		onDetach
	}: {
		activeDraftId: string;
		activeCell: { categoryIdx: number; point: number } | null;
		activeQuestionId: string | null;
		onPickQuestion: () => void;
		onCreateNew: () => void;
		onDetach: () => void;
	} = $props();

	const draft = $derived(pool.getDraft(activeDraftId));
	const cell = $derived(
		activeCell && draft
			? draft.board.categories[activeCell.categoryIdx]?.questions[activeCell.point] ?? null
			: null
	);
	const question = $derived(
		cell ? pool.getQuestion(cell.questionId) : activeQuestionId ? pool.getQuestion(activeQuestionId) : null
	);
	const refs = $derived(question ? pool.referenceLocations(question.id) : []);

	function setField<K extends keyof NonNullable<typeof question>>(
		key: K,
		value: NonNullable<typeof question>[K]
	): void {
		if (!question) return;
		pool.updateQuestion(question.id, { [key]: value } as Partial<NonNullable<typeof question>>);
	}

	function addChoice(): void {
		if (!question) return;
		const next = [
			...(question.choices ?? []),
			{ id: `c${(question.choices?.length ?? 0) + 1}`, text: '', correct: false }
		];
		pool.updateQuestion(question.id, { choices: next });
	}
	function removeChoice(i: number): void {
		if (!question?.choices) return;
		const next = question.choices.filter((_, idx) => idx !== i);
		pool.updateQuestion(question.id, { choices: next });
	}
</script>

<section class="editor">
	<header class="ed-head">
		<div class="h-left">
			<div class="kicker">Question</div>
			{#if question && activeCell && cell?.questionId === question.id && draft}
				{@const cat = draft.board.categories[activeCell.categoryIdx]?.name ?? ''}
				<div class="attach-row">
					<span class="attach-info">
						Attached to <strong>{cat}</strong> · {activeCell.point} pts
					</span>
					<span class="sep">·</span>
					<button class="link" onclick={onPickQuestion}>Replace</button>
					<span class="sep">·</span>
					<button class="link link-danger" onclick={onDetach}>Detach</button>
				</div>
			{/if}
			{#if question}
				<div class="id-row">
					<code class="qid" class:draft={isDraftId(question.id)}>{question.id}</code>
					{#if isDraftId(question.id)}
						<span class="badge-draft" title="Auto-renames to q_&lt;slug&gt; when prompt has text and the question is unreferenced">
							draft id · will rename
						</span>
					{/if}
					{#if question.previousIds?.length}
						<span class="prev" title="Previous ids">
							renamed from {question.previousIds[question.previousIds.length - 1]}
						</span>
					{/if}
				</div>
			{/if}
		</div>
		{#if question}
			<div class="status status-{question.status}">{question.status}</div>
		{/if}
	</header>

	<div class="ed-body">
		{#if !question}
			{#if activeCell}
				<div class="empty">
					<h3>Empty cell</h3>
					<p>This board cell has no question. Pick an existing one from the pool, or create a new question.</p>
					<div class="empty-actions">
						<Button onclick={onPickQuestion}>Pick existing question</Button>
						<Button variant="secondary" onclick={onCreateNew}>+ Create new question</Button>
					</div>
				</div>
			{:else}
				<div class="empty">
					<h3>No question selected</h3>
					<p>Select a board cell on the left, or create a new question from the top bar.</p>
					<div class="empty-actions">
						<Button onclick={onCreateNew}>+ Create new question</Button>
					</div>
				</div>
			{/if}
		{:else}
			<div class="grid-2">
				<label>
					<span>Prompt</span>
					<textarea
						rows="3"
						value={question.prompt}
						oninput={(e) => setField('prompt', (e.target as HTMLTextAreaElement).value)}
						placeholder="e.g. In welchem Jahr fiel die Berliner Mauer?"
					></textarea>
				</label>
				<label>
					<span>Kind</span>
					<select
						value={question.kind}
						onchange={(e) => {
							const next = (e.target as HTMLSelectElement).value as typeof question.kind;
							setField('kind', next);
							if (next === 'numeric' && question.answerNumeric === undefined) {
								setField('answerNumeric', Number(question.answer) || 0);
								setField('numericInput', { tolerance: 0 });
							}
						}}
					>
						<option value="text">Text</option>
						<option value="numeric">Numeric</option>
						<option value="order">Order</option>
					</select>
				</label>
				{#if question.kind === 'numeric'}
					<label>
						<span>Exact answer</span>
						<input
							type="number"
							step="any"
							value={question.answerNumeric ?? 0}
							oninput={(e) =>
								setField('answerNumeric', Number((e.target as HTMLInputElement).value))}
						/>
					</label>
					<label>
						<span>Unit <small>(optional)</small></span>
						<input
							value={question.unit ?? ''}
							oninput={(e) => setField('unit', (e.target as HTMLInputElement).value)}
							placeholder="e.g. km, °C"
						/>
					</label>
				{:else}
					<label>
						<span>Answer</span>
						<input
							value={question.answer}
							oninput={(e) => setField('answer', (e.target as HTMLInputElement).value)}
							placeholder="e.g. 1989"
						/>
					</label>
				{/if}
				<label>
					<span>Default language</span>
					<select
						value={question.defaultLang}
						onchange={(e) =>
							setField(
								'defaultLang',
								(e.target as HTMLSelectElement).value as typeof question.defaultLang
							)}
					>
						<option value="de">Deutsch</option>
						<option value="en">English</option>
					</select>
				</label>
				<label class="full">
					<span>Explanation <small>(optional)</small></span>
					<textarea
						rows="2"
						value={question.explanation ?? ''}
						oninput={(e) =>
							setField('explanation', (e.target as HTMLTextAreaElement).value)}
					></textarea>
				</label>
			</div>

			{#if question.kind === 'numeric'}
				<section class="variants">
					<header><h3>Numeric answer shapes</h3></header>

					<div class="variant-row">
						<header class="v-head">
							<label class="v-toggle">
								<input
									type="checkbox"
									checked={!!question.numericInput}
									onchange={(e) => {
										if ((e.target as HTMLInputElement).checked) {
											pool.updateQuestion(question.id, { numericInput: { tolerance: 0 } });
										} else {
											pool.updateQuestion(question.id, { numericInput: null });
										}
									}}
								/>
								<strong>Numeric input (exact ± tolerance)</strong>
							</label>
						</header>
						{#if question.numericInput}
							<div class="v-body">
								<label>
									<span>Tolerance</span>
									<input
										type="number"
										step="any"
										min="0"
										value={question.numericInput.tolerance}
										oninput={(e) => {
											if (!question?.numericInput) return;
											pool.updateQuestion(question.id, {
												numericInput: {
													tolerance: Math.max(0, Number((e.target as HTMLInputElement).value))
												}
											});
										}}
									/>
								</label>
								<p class="v-hint">
									Accept {question.answerNumeric ?? 0} ± {question.numericInput.tolerance}
									{question.unit ?? ''}
								</p>
							</div>
						{/if}
					</div>

					<div class="variant-row">
						<header class="v-head">
							<label class="v-toggle">
								<input
									type="checkbox"
									checked={!!question.range}
									onchange={(e) => {
										if ((e.target as HTMLInputElement).checked) {
											const v = question.answerNumeric ?? 0;
											pool.updateQuestion(question.id, {
												range: { min: v - 5, max: v + 5, step: 1, tolerance: 0 }
											});
										} else {
											pool.updateQuestion(question.id, { range: null });
										}
									}}
								/>
								<strong>Range (min, max, step, tolerance)</strong>
							</label>
						</header>
						{#if question.range}
							<div class="v-body">
								<div class="v-row">
									<label>
										<span>Min</span>
										<input
											type="number"
											step="any"
											value={question.range.min}
											oninput={(e) => {
												if (!question?.range) return;
												pool.updateQuestion(question.id, {
													range: { ...question.range, min: Number((e.target as HTMLInputElement).value) }
												});
											}}
										/>
									</label>
									<label>
										<span>Max</span>
										<input
											type="number"
											step="any"
											value={question.range.max}
											oninput={(e) => {
												if (!question?.range) return;
												pool.updateQuestion(question.id, {
													range: { ...question.range, max: Number((e.target as HTMLInputElement).value) }
												});
											}}
										/>
									</label>
									<label>
										<span>Step</span>
										<input
											type="number"
											step="any"
											min="0"
											value={question.range.step}
											oninput={(e) => {
												if (!question?.range) return;
												pool.updateQuestion(question.id, {
													range: {
														...question.range,
														step: Math.max(0, Number((e.target as HTMLInputElement).value))
													}
												});
											}}
										/>
									</label>
									<label>
										<span>Tolerance</span>
										<input
											type="number"
											step="any"
											min="0"
											value={question.range.tolerance}
											oninput={(e) => {
												if (!question?.range) return;
												pool.updateQuestion(question.id, {
													range: {
														...question.range,
														tolerance: Math.max(0, Number((e.target as HTMLInputElement).value))
													}
												});
											}}
										/>
									</label>
								</div>
								{#if question.range.max <= question.range.min}
									<p class="v-err">⚠ max must be greater than min</p>
								{:else}
									<p class="v-hint">
										Accept any value in [{question.range.min}, {question.range.max}]
										(step {question.range.step}, ± {question.range.tolerance})
										{question.unit ?? ''}
									</p>
								{/if}
							</div>
						{/if}
					</div>
				</section>
			{/if}

			{#if question.choices}
				<section class="choices">
					<header class="ch-head">
						<h3>Choices</h3>
						<Button size="sm" variant="ghost" onclick={addChoice}>+ Add</Button>
					</header>
					<ul class="ch-list">
						{#each question.choices as c, i (c.id)}
							<li class="ch">
								<button
									class="ch-correct"
									class:on={c.correct}
									aria-label={c.correct ? 'Mark as incorrect' : 'Mark as correct'}
									onclick={() => {
										if (!question) return;
										const next = question.choices!.map((x, idx) =>
											idx === i ? { ...x, correct: !x.correct } : x
										);
										pool.updateQuestion(question.id, { choices: next });
									}}
								>
									{c.correct ? '✓' : '○'}
								</button>
								<input
									value={c.text}
									oninput={(e) => {
										if (!question?.choices) return;
										const next = question.choices.map((x, idx) =>
											idx === i ? { ...x, text: (e.target as HTMLInputElement).value } : x
										);
										pool.updateQuestion(question.id, { choices: next });
									}}
									placeholder="Choice text"
								/>
								<button class="ch-rm" aria-label="Remove" onclick={() => removeChoice(i)}>✕</button>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			<footer class="refs">
				{#if refs.length}
					<h3>Referenced by</h3>
					<ul>
						{#each refs as r (r.draft.id + r.category + r.point)}
							<li>
								<strong>{r.draft.title}</strong> · {r.category} · {r.point} pts
							</li>
						{/each}
					</ul>
				{:else}
					<p class="muted">Not yet referenced by any draft.</p>
				{/if}
			</footer>
		{/if}
	</div>
</section>

<style>
	.editor {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.ed-head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		padding: var(--space-3) var(--space-4);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
		gap: var(--space-3);
	}
	.kicker {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.id-row {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		margin-top: 2px;
		flex-wrap: wrap;
	}
	.qid {
		font-family: var(--font-mono);
		font-size: calc(0.85rem * var(--font-scale));
		background: var(--bg-primary);
		padding: 2px var(--space-2);
		border-radius: var(--radius-sm);
		color: var(--color-text);
	}
	.qid.draft {
		background: color-mix(in srgb, var(--color-accent) 18%, var(--bg-primary));
		color: var(--color-accent);
	}
	.badge-draft {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-accent);
		background: color-mix(in srgb, var(--color-accent) 12%, transparent);
		padding: 1px var(--space-2);
		border-radius: var(--radius-full);
	}
	.prev {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.status {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 2px var(--space-2);
		border-radius: var(--radius-full);
	}
	.status-draft {
		background: color-mix(in srgb, var(--color-accent) 18%, transparent);
		color: var(--color-accent);
	}
	.status-named {
		background: color-mix(in srgb, var(--color-success) 18%, transparent);
		color: var(--color-success);
	}
	.status-referenced {
		background: color-mix(in srgb, var(--color-primary) 18%, transparent);
		color: var(--color-primary);
	}
	.status-deprecated {
		background: color-mix(in srgb, var(--color-danger) 18%, transparent);
		color: var(--color-danger);
	}
	.ed-body {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	.h-left {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
	}
	.attach-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
		flex-wrap: wrap;
	}
	.attach-info strong {
		color: var(--color-text);
	}
	.attach-row .sep {
		opacity: 0.5;
	}
	.link {
		background: transparent;
		border: none;
		color: var(--color-primary);
		font-family: var(--font-body);
		font-size: calc(0.8rem * var(--font-scale));
		font-weight: 600;
		cursor: pointer;
		padding: 0;
	}
	.link:hover {
		text-decoration: underline;
	}
	.link-danger {
		color: var(--color-danger);
	}
	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		text-align: center;
		gap: var(--space-3);
		padding: var(--space-8);
	}
	.empty h3 {
		font-family: var(--font-heading);
		margin: 0;
	}
	.empty p {
		color: var(--color-text-muted);
		max-width: 32rem;
		margin: 0;
	}
	.empty-actions {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-3);
	}
	.grid-2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	label.full {
		grid-column: 1 / -1;
	}
	label > span {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	label small {
		text-transform: none;
		opacity: 0.7;
	}
	input,
	textarea,
	select {
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.variants {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.variants > header h3 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.variant-row {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		background: var(--bg-primary);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
	}
	.v-head {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.v-toggle {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		font-size: calc(0.85rem * var(--font-scale));
		cursor: pointer;
	}
	.v-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.v-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--space-2);
	}
	.v-hint {
		margin: 0;
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.v-err {
		margin: 0;
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-danger);
	}
	.choices {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.ch-head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.ch-head h3 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.ch-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.ch {
		display: grid;
		grid-template-columns: 2.5rem 1fr 2rem;
		gap: var(--space-2);
		align-items: center;
	}
	.ch-correct {
		background: var(--bg-primary);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		width: 2.25rem;
		height: 2.25rem;
		cursor: pointer;
		font-family: var(--font-mono);
		color: var(--color-text-muted);
	}
	.ch-correct.on {
		background: var(--color-success);
		color: var(--color-text-inverse);
		border-color: var(--color-success);
	}
	.ch-rm {
		background: transparent;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.refs {
		border-top: var(--border-width) var(--border-style) var(--border-color);
		padding-top: var(--space-3);
	}
	.refs h3 {
		font-family: var(--font-body);
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin: 0 0 var(--space-2) 0;
	}
	.refs ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.muted {
		color: var(--color-text-muted);
		font-size: calc(0.85rem * var(--font-scale));
		margin: 0;
	}
</style>
