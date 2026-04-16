import { SvelteSet } from "svelte/reactivity";

export class WindowState {
  poppedOutInspectors = new SvelteSet<string>();
  isScriptsPoppedOut = $state(false);

  toggleInspector(id: string, popped: boolean) {
    if (popped) this.poppedOutInspectors.add(id);
    else this.poppedOutInspectors.delete(id);
  }

  toggleScripts(popped: boolean) {
    this.isScriptsPoppedOut = popped;
  }
}

export const windowState = new WindowState();
