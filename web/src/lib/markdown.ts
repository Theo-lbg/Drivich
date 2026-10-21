// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// Small Markdown renderer for the preview of .md files: headings, emphasis, code,
// lists, quotes, rules and links. Everything is HTML-escaped first, so the output
// can never contain markup that the author did not get from this renderer.

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

function inline(raw: string): string {
	const codes: string[] = [];
	let s = esc(raw).replace(/`([^`]+)`/g, (_, c) => `\u0000${codes.push(`<code>${c}</code>`) - 1}\u0000`);
	s = s
		.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, text, href) =>
			/^(https?:|mailto:|\/|#)/i.test(href) ? `<a href="${href}" target="_blank" rel="noopener noreferrer">${text}</a>` : text
		)
		.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
		.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
		.replace(/~~([^~]+)~~/g, '<del>$1</del>');
	return s.replace(/\u0000(\d+)\u0000/g, (_, i) => codes[+i]);
}

export function renderMarkdown(src: string): string {
	const lines = src.replace(/\r\n?/g, '\n').split('\n');
	const out: string[] = [];
	let list: 'ul' | 'ol' | null = null;
	let para: string[] = [];
	let fence: string[] | null = null;

	const flushPara = () => {
		if (para.length) out.push(`<p>${inline(para.join(' '))}</p>`);
		para = [];
	};
	const closeList = () => {
		if (list) out.push(`</${list}>`);
		list = null;
	};

	for (const line of lines) {
		if (fence) {
			if (/^```/.test(line)) {
				out.push(`<pre><code>${esc(fence.join('\n'))}</code></pre>`);
				fence = null;
			} else fence.push(line);
			continue;
		}
		if (/^```/.test(line)) {
			flushPara();
			closeList();
			fence = [];
			continue;
		}
		const h = /^(#{1,6})\s+(.*)$/.exec(line);
		const li = /^\s*([-*+]|\d+\.)\s+(.*)$/.exec(line);
		if (h) {
			flushPara();
			closeList();
			out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`);
		} else if (li) {
			flushPara();
			const kind = /\d/.test(li[1]) ? 'ol' : 'ul';
			if (list !== kind) {
				closeList();
				out.push(`<${kind}>`);
				list = kind;
			}
			out.push(`<li>${inline(li[2])}</li>`);
		} else if (/^\s*(---+|\*\*\*+)\s*$/.test(line)) {
			flushPara();
			closeList();
			out.push('<hr>');
		} else if (/^>\s?/.test(line)) {
			flushPara();
			closeList();
			out.push(`<blockquote>${inline(line.replace(/^>\s?/, ''))}</blockquote>`);
		} else if (!line.trim()) {
			flushPara();
			closeList();
		} else {
			closeList();
			para.push(line.trim());
		}
	}
	if (fence) out.push(`<pre><code>${esc(fence.join('\n'))}</code></pre>`);
	flushPara();
	closeList();
	return out.join('\n');
}
