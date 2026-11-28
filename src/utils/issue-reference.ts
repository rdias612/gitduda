export interface IssueTriggerMatch { start: number; end: number; query: string }

export function detectIssueTrigger(text: string, cursorPos: number): IssueTriggerMatch | null {
  const match = /(?:^|\s)#([^\s#]*)$/.exec(text.slice(0, cursorPos));
  if (!match) return null;
  return { start: cursorPos - match[1].length - 1, end: cursorPos, query: match[1] };
}

export function stripIssueTrigger(text: string, trigger: IssueTriggerMatch): string {
  const before = text.slice(0, trigger.start);
  const after = text.slice(trigger.end);
  if (!after || after.startsWith('\n')) return before.replace(/[ \t]+$/, '') + after;
  if (!before || before.endsWith('\n')) return before + after.replace(/^[ \t]+/, '');
  if (/^[ \t]/.test(after)) return before.replace(/[ \t]+$/, '') + after;
  return before + after;
}

export function appendIssueReference(description: string, reference: string): string {
  const body = description.trimEnd();
  if (body.split(/\r?\n/).some(line => line.trim().toLowerCase() === reference.toLowerCase())) return description;
  if (!body) return reference;
  const lastParagraph = body.split(/\r?\n\s*\n/).pop()!;
  const separator = lastParagraph.split(/\r?\n/).every(line => /^(?:Refs|Closes) \S+$/i.test(line)) ? '\n' : '\n\n';
  return body + separator + reference;
}
