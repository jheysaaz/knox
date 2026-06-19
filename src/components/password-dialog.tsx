import { Eye, EyeOff, Lock, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';

interface PasswordDialogProps {
  open: boolean;
  fileNames: string[];
  onConfirm: (password: string) => void;
  onCancel: () => void;
}

export function PasswordDialog({
  open,
  fileNames,
  onConfirm,
  onCancel,
}: PasswordDialogProps) {
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setPassword('');
      setShowPassword(false);
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [open]);

  if (!open) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (password.trim()) {
      onConfirm(password.trim());
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <Card className="w-full max-w-sm shadow-xl ring-1 ring-foreground/10">
        <CardContent className="pt-6 pb-6">
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-2">
                <Lock className="size-5 text-muted-foreground" />
                <h2 className="text-base font-semibold">
                  PDF Password Required
                </h2>
              </div>
              <button
                type="button"
                onClick={onCancel}
                className="p-1 text-muted-foreground hover:text-foreground transition-colors"
              >
                <X className="size-4" />
              </button>
            </div>

            <p className="text-sm text-muted-foreground">
              {fileNames.length === 1
                ? `"${fileNames[0]}" is password-protected. Enter the password to process this file.`
                : `${fileNames.length} files are password-protected. Enter the password to process them.`}
            </p>

            {fileNames.length > 1 && (
              <ul className="text-xs text-muted-foreground space-y-0.5 max-h-20 overflow-y-auto">
                {fileNames.map((name) => (
                  <li key={name} className="truncate">
                    • {name}
                  </li>
                ))}
              </ul>
            )}

            <div className="relative">
              <input
                ref={inputRef}
                type={showPassword ? 'text' : 'password'}
                placeholder="Enter PDF password"
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 pr-8 text-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              <button
                type="button"
                className="absolute right-1 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
                onClick={() => setShowPassword(!showPassword)}
                tabIndex={-1}
              >
                {showPassword ? (
                  <EyeOff className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
              </button>
            </div>

            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={onCancel}
              >
                Skip File
              </Button>
              <Button type="submit" size="sm" disabled={!password.trim()}>
                Confirm
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
