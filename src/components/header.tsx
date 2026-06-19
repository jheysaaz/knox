import { useCallback, useEffect, useState } from "react";
import { Sun, Moon, Settings, Bug, History } from "lucide-react";
import { Button } from "@/components/ui/button";

/** Header component with greeting, theme toggle, and activity panel toggle. */
interface HeaderProps {
  greeting: string;
  showActivity: boolean;
  onToggleActivity: () => void;
  showHistory: boolean;
  onToggleHistory: () => void;
}

/** Top bar with greeting, theme toggle, activity toggle, and settings button. */
export function Header({
  greeting,
  showActivity,
  onToggleActivity,
  showHistory,
  onToggleHistory,
}: HeaderProps) {
  const [isDark, setIsDark] = useState(() => {
    const stored = localStorage.getItem("theme");
    if (stored) return stored === "dark";
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  });

  useEffect(() => {
    const root = document.documentElement;
    if (isDark) {
      root.classList.add("dark");
      localStorage.setItem("theme", "dark");
    } else {
      root.classList.remove("dark");
      localStorage.setItem("theme", "light");
    }
  }, [isDark]);

  const toggleTheme = useCallback(() => {
    setIsDark((prev) => !prev);
  }, []);

  return (
    <div data-tauri-drag-region="deep">
      <div className="flex items-center justify-between px-6 py-4">
        <div>
          <h1 className="text-xl font-bold text-foreground">Hi,</h1>
          <h2 className="text-muted-foreground">{greeting}</h2>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            onClick={onToggleHistory}
            title={showHistory ? "Hide history" : "Show history"}
          >
            <History className="h-5 w-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={onToggleActivity}
            title={showActivity ? "Hide activity" : "Show activity"}
          >
            <Bug className="h-5 w-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleTheme}
            title="Toggle theme"
          >
            {isDark ? (
              <Sun className="h-5 w-5" />
            ) : (
              <Moon className="h-5 w-5" />
            )}
          </Button>
          <Button variant="ghost" size="icon" title="Settings">
            <Settings className="h-5 w-5" />
          </Button>
        </div>
      </div>
    </div>
  );
}
