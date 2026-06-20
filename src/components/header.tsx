import { getCurrentWindow } from '@tauri-apps/api/window';
import { Moon, Settings, Sun } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';

interface HeaderProps {
  greeting: string;
}

function getInitialTheme(): boolean {
  const stored = localStorage.getItem('theme');
  if (stored) return stored === 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export function Header({ greeting }: HeaderProps) {
  const [isDark, setIsDark] = useState(getInitialTheme);

  useEffect(() => {
    const root = document.documentElement;
    if (isDark) {
      root.classList.add('dark');
      localStorage.setItem('theme', 'dark');
    } else {
      root.classList.remove('dark');
      localStorage.setItem('theme', 'light');
    }
  }, [isDark]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    (async () => {
      try {
        const w = getCurrentWindow();
        const native = await w.theme();
        if (native) {
          setIsDark((prev) => {
            if (prev !== (native === 'dark')) return native === 'dark';
            return prev;
          });
        }

        const fn = await w.onThemeChanged(({ payload: theme }) => {
          setIsDark(theme === 'dark');
        });
        unlisten = fn;
      } catch {
        // Tauri APIs unavailable (e.g. browser dev, tests)
      }
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem('theme')) {
        setIsDark(e.matches);
      }
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  const toggleTheme = useCallback(() => {
    setIsDark((prev) => !prev);
  }, []);

  return (
    <div className="flex items-center justify-between px-6 py-4">
      <div data-tauri-drag-region="deep">
        <h1 className="text-xl font-bold text-foreground">Hi,</h1>
        <h2 className="text-muted-foreground">{greeting}</h2>
      </div>
      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleTheme}
          title="Toggle theme"
        >
          {isDark ? <Sun className="h-5 w-5" /> : <Moon className="h-5 w-5" />}
        </Button>
        <Button variant="ghost" size="icon" title="Settings">
          <Settings className="h-5 w-5" />
        </Button>
      </div>
    </div>
  );
}
