import { ChangeSet, EditorState, Transaction, type TransactionSpec } from "@codemirror/state";
import { resolveImageUrls } from "./EditorSessionStore";
import { contentChange } from "./editorChanges";

function imageChanges(content: string, replacements: Record<string, string>) {
  const changes: Array<{ from: number; to: number; insert: string }> = [];
  for (const [local, remote] of Object.entries(replacements)) {
    const escaped = local.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const pattern = new RegExp(`(!\\[[^\\]\\r\\n]*\\]\\(\\s*)${escaped}(?=(?:\\s+['\"][^'\"]*['\"])?\\s*\\))`, "g");
    for (const match of content.matchAll(pattern)) {
      const from = match.index + match[1].length;
      changes.push({ from, to: from + local.length, insert: remote });
    }
  }
  return ChangeSet.of(changes.sort((a, b) => a.from - b.from), content.length);
}

// Undo may contain the temporary URL from before the upload. Normalize it within
// that same transaction, before it reaches the store or can be saved again.
export function resolvedImageHistory(replacements: () => Record<string, string>) {
  return EditorState.transactionFilter.of((transaction) => {
    if (!transaction.docChanged) return transaction;
    const content = transaction.newDoc.toString();
    const resolved = resolveImageUrls(content, replacements());
    if (resolved === content) return transaction;
    return [transaction, {
      changes: imageChanges(content, replacements()),
      sequential: true,
      annotations: Transaction.addToHistory.of(false)
    }];
  });
}

export function externalEditorChange(before: string, after: string, replacements: Record<string, string>): TransactionSpec {
  const systemUpdate = resolveImageUrls(before, replacements) === after;
  return {
    changes: systemUpdate ? imageChanges(before, replacements) : contentChange(before, after),
    annotations: Transaction.addToHistory.of(!systemUpdate)
  };
}
