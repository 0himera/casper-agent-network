import React from "react";

const r = React as any;
if (r) {
  const clientInternals = r.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
  if (clientInternals && !r.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED) {
    const ReactDebugCurrentFrame = {
      getStackAddendum() {
        return "";
      },
      setExtraStackFrame() {},
      getCurrentStack() {
        return null;
      }
    };

    const secretInternals = {
      ReactDebugCurrentFrame,
      get ReactCurrentDispatcher() {
        return {
          get current() {
            return clientInternals.H;
          },
          set current(value) {
            clientInternals.H = value;
          }
        };
      },
      get ReactCurrentOwner() {
        return {
          get current() {
            return clientInternals.A;
          },
          set current(value) {
            clientInternals.A = value;
          }
        };
      },
      get ReactCurrentBatchConfig() {
        return {
          get current() {
            return clientInternals.T;
          },
          set current(value) {
            clientInternals.T = value;
          }
        };
      }
    };

    Object.defineProperty(r, "__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED", {
      value: secretInternals,
      configurable: true,
      enumerable: true,
      writable: true
    });
  }
}

// Intercept Symbol.for("react.element") to return "react.transitional.element" during module loading
if (!(globalThis as any).__originalSymbolFor) {
  (globalThis as any).__originalSymbolFor = Symbol.for;
  Symbol.for = function (key) {
    if (key === "react.element") {
      return (globalThis as any).__originalSymbolFor("react.transitional.element");
    }
    return (globalThis as any).__originalSymbolFor(key);
  };
}
