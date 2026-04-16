type KeyBinding = {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  /** If true, fires even when an input/textarea is focused */
  global?: boolean;
  action: (e: KeyboardEvent) => void;
};

export class Keymap {
  private bindings: KeyBinding[] = [];
  private handler: (e: KeyboardEvent) => void;

  constructor() {
    this.handler = (e: KeyboardEvent) => this.onKeydown(e);
  }

  /** Register a keybinding. Returns `this` for chaining. */
  bind(key: string, action: (e: KeyboardEvent) => void, opts?: { ctrl?: boolean; shift?: boolean; global?: boolean }): this {
    this.bindings.push({ key, action, ...opts });
    return this;
  }

  /** Attach the keydown listener to the document. Call in onMount. */
  attach(): () => void {
    document.addEventListener("keydown", this.handler);
    return () => document.removeEventListener("keydown", this.handler);
  }

  private onKeydown(e: KeyboardEvent) {
    const inInput = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;

    for (const b of this.bindings) {
      if (!b.global && inInput) continue;
      if (b.ctrl && !(e.ctrlKey || e.metaKey)) continue;
      if (b.shift && !e.shiftKey) continue;
      if (e.key.toLowerCase() !== b.key.toLowerCase()) continue;

      e.preventDefault();
      b.action(e);
      return;
    }
  }
}
