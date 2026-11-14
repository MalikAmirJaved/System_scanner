import { useEffect, useState } from "react";
import { Moon, Sun } from "lucide-react";
import { Button } from "./ui/button";

type Theme = "light" | "dark";

const STORAGE_KEY = "system-scanner.theme";

function initialTheme(): Theme {
  if (typeof window === "undefined") return "light";

  try {
    return window.localStorage.getItem(STORAGE_KEY) === "dark" ? "dark" : "light";
  } catch {
    // Storage can be denied; the app still works with the light default.
    return "light";
  }
}

/**
 * Manual light/dark switch. The palette lives in `globals.css` under
 * `[data-theme="dark"]`; this only selects it and remembers the choice locally.
 * Following the OS preference is not implemented (F-009).
 */
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(initialTheme);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;

    try {
      window.localStorage.setItem(STORAGE_KEY, theme);
    } catch {
      // A denied write costs persistence, not correctness.
    }
  }, [theme]);

  const next: Theme = theme === "dark" ? "light" : "dark";

  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => setTheme(next)}
      aria-label={`Switch to ${next} theme`}
    >
      {theme === "dark" ? (
        <Sun aria-hidden="true" className="size-3.5" />
      ) : (
        <Moon aria-hidden="true" className="size-3.5" />
      )}
      <span aria-hidden="true">{next === "dark" ? "Dark" : "Light"}</span>
    </Button>
  );
}
