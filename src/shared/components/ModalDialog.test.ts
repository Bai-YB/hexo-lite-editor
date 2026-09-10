import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import ModalDialog from "./ModalDialog.svelte";
import { hasOpenModal } from "./modalStack";

Element.prototype.animate = vi.fn(() => ({ cancel() {}, finish() {}, onfinish: null, finished: Promise.resolve() })) as unknown as typeof Element.prototype.animate;

afterEach(cleanup);
describe("modal keyboard ownership", () => {
  it("only closes the top modal and unregisters when destroyed", async () => {
    const first = vi.fn();
    const second = vi.fn();
    render(ModalDialog, { title: "First", onClose: first });
    const top = render(ModalDialog, { title: "Second", onClose: second });
    expect(hasOpenModal()).toBe(true);
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
    top.unmount();
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(first).toHaveBeenCalledTimes(1);
    cleanup();
    expect(hasOpenModal()).toBe(false);
  });
  it("does not close while confirming an input-method composition", async () => {
    const close = vi.fn();
    render(ModalDialog, { title: "Compose", onClose: close });
    await fireEvent.keyDown(window, { key: "Escape", isComposing: true });
    expect(close).not.toHaveBeenCalled();
  });
});
