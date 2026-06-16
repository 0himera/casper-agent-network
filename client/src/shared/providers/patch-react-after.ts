// Restore the original Symbol.for to prevent any global side effects
if ((globalThis as any).__originalSymbolFor) {
  Symbol.for = (globalThis as any).__originalSymbolFor;
}
export {};
