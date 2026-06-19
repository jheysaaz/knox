import { LoaderCircle } from 'lucide-react';
import { cn } from '@/lib/utils';

export function Spinner({ className }: { className?: string }) {
  return (
    <div className={cn('flex items-center justify-center py-12', className)}>
      <LoaderCircle className="size-6 animate-spin text-muted-foreground" />
    </div>
  );
}
