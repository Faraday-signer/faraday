import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode
} from "react";

/**
 * Route union for the side panel surface. Keeping it a tagged union means the
 * compiler knows which screens exist and can narrow props per route.
 */
export type Route =
  | { name: "home" }
  | { name: "onboarding" }
  | { name: "pair-scan" }
  | { name: "pair-paste" }
  | { name: "receive" }
  | { name: "send-compose" }
  | { name: "send-review"; draft: SendDraft }
  | { name: "send-sign"; draft: SendDraft; txBase64: string }
  | { name: "settings" }
  | { name: "settings-device" }
  | { name: "settings-origins" }
  | { name: "settings-network" }
  | { name: "settings-about" };

export interface SendDraft {
  mint: string;
  symbol: string;
  decimals: number;
  amountUi: string;
  recipient: string;
}

export type RouteName = Route["name"];

interface NavigationContextValue {
  current: Route;
  stack: Route[];
  push: (route: Route) => void;
  replace: (route: Route) => void;
  back: () => void;
  reset: (route: Route) => void;
  canGoBack: boolean;
}

const NavigationContext = createContext<NavigationContextValue | null>(null);

interface NavigationProviderProps {
  initial: Route;
  children: ReactNode;
}

export function NavigationProvider({ initial, children }: NavigationProviderProps) {
  const [stack, setStack] = useState<Route[]>(() => [initial]);

  const push = useCallback((route: Route) => {
    setStack((prev) => [...prev, route]);
  }, []);

  const replace = useCallback((route: Route) => {
    setStack((prev) => {
      if (prev.length === 0) return [route];
      const copy = prev.slice(0, -1);
      copy.push(route);
      return copy;
    });
  }, []);

  const back = useCallback(() => {
    setStack((prev) => (prev.length > 1 ? prev.slice(0, -1) : prev));
  }, []);

  const reset = useCallback((route: Route) => {
    setStack([route]);
  }, []);

  const value = useMemo<NavigationContextValue>(() => {
    const current = stack[stack.length - 1] ?? initial;
    return {
      current,
      stack,
      push,
      replace,
      back,
      reset,
      canGoBack: stack.length > 1
    };
  }, [stack, initial, push, replace, back, reset]);

  return <NavigationContext.Provider value={value}>{children}</NavigationContext.Provider>;
}

export function useNavigation(): NavigationContextValue {
  const ctx = useContext(NavigationContext);
  if (!ctx) {
    throw new Error("useNavigation must be used inside a NavigationProvider.");
  }
  return ctx;
}

/**
 * Narrow the current route to a given tag. Lets consumers read props off the
 * route safely. Returns null when the active route isn't the requested one.
 */
export function useRouteOf<T extends RouteName>(name: T) {
  const { current } = useNavigation();
  if (current.name !== name) {
    return null;
  }
  return current as Extract<Route, { name: T }>;
}
