/** Own event subscriptions as one retryable connection, including partial setup. */
export function createListenerConnection(
  registrations: (() => Promise<() => void>)[],
  reportError: (error: string | null) => void,
) {
  let disposed = false;
  let connected = false;
  let pending: Promise<boolean> | null = null;
  let cleanups: (() => void)[] = [];

  async function establish() {
    const acquired: (() => void)[] = [];
    try {
      for (const register of registrations) {
        acquired.push(await register());
        if (disposed) {
          for (const cleanup of acquired) cleanup();
          return false;
        }
      }
      cleanups = acquired;
      connected = true;
      reportError(null);
      return true;
    } catch (error) {
      for (const cleanup of acquired) cleanup();
      if (!disposed) reportError(`Could not connect to metrics: ${String(error)}`);
      return false;
    }
  }

  return {
    connect(): Promise<boolean> {
      if (disposed) return Promise.resolve(false);
      if (connected) return Promise.resolve(true);
      if (!pending) pending = establish().finally(() => { pending = null; });
      return pending;
    },
    dispose() {
      disposed = true;
      for (const cleanup of cleanups) cleanup();
      cleanups = [];
    },
  };
}
