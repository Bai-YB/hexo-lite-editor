import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import { history, undo, redo } from "@codemirror/commands";
import { externalEditorChange, resolvedImageHistory } from "./imageHistory";

describe("uploaded image history", () => {
  it("undoes user typing without restoring expired image URLs", () => {
    const local = "http://hlex-asset.localhost/0f5845c7-a9d8-40e9-97af-f770331f5000";
    const remote = "https://images.example/a.png";
    let replacements: Record<string, string> = {};
    let state = EditorState.create({ doc: "body", extensions: [history(), resolvedImageHistory(() => replacements)] });
    state = state.update({ changes: { from: 4, insert: `\n![photo](${local})` }, userEvent: "input" }).state;
    replacements = { [local]: remote };
    state = state.update(externalEditorChange(state.doc.toString(), `body\n![photo](${remote})`, replacements)).state;
    const target = { get state() { return state; }, dispatch(transaction: import("@codemirror/state").Transaction) { state = transaction.state; } };
    expect(undo(target)).toBe(true);
    expect(state.doc.toString()).not.toContain(local);
    expect(redo(target)).toBe(true);
    expect(state.doc.toString()).toBe(`body\n![photo](${remote})`);
  });

  it("keeps edits to image descriptions undoable across the upload", () => {
    const local = "http://hlex-asset.localhost/0f5845c7-a9d8-40e9-97af-f770331f5000";
    const remote = "https://images.example/a.png";
    let replacements: Record<string, string> = {};
    let state = EditorState.create({ doc: `![old](${local})`, extensions: [history(), resolvedImageHistory(() => replacements)] });
    state = state.update({ changes: { from: 2, to: 5, insert: "new description" }, userEvent: "input" }).state;
    replacements = { [local]: remote };
    state = state.update(externalEditorChange(state.doc.toString(), `![new description](${remote})`, replacements)).state;
    const target = { get state() { return state; }, dispatch(transaction: import("@codemirror/state").Transaction) { state = transaction.state; } };
    expect(undo(target)).toBe(true);
    expect(state.doc.toString()).toBe(`![old](${remote})`);
    expect(redo(target)).toBe(true);
    expect(state.doc.toString()).toBe(`![new description](${remote})`);
  });
});
