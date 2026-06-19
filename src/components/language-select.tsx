import { Check, ChevronDown, Search, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { cn } from '@/lib/utils';

const LANGUAGES: { code: string; name: string }[] = [
  { code: 'afr', name: 'Afrikaans' },
  { code: 'amh', name: 'Amharic' },
  { code: 'ara', name: 'Arabic' },
  { code: 'aze', name: 'Azerbaijani' },
  { code: 'bel', name: 'Belarusian' },
  { code: 'ben', name: 'Bengali' },
  { code: 'bul', name: 'Bulgarian' },
  { code: 'cat', name: 'Catalan' },
  { code: 'ces', name: 'Czech' },
  { code: 'chi_sim', name: 'Chinese (Simplified)' },
  { code: 'chi_tra', name: 'Chinese (Traditional)' },
  { code: 'cjk', name: 'CJK (Chinese/Japanese/Korean)' },
  { code: 'dan', name: 'Danish' },
  { code: 'deu', name: 'German' },
  { code: 'ell', name: 'Greek' },
  { code: 'eng', name: 'English' },
  { code: 'epo', name: 'Esperanto' },
  { code: 'est', name: 'Estonian' },
  { code: 'eus', name: 'Basque' },
  { code: 'fas', name: 'Persian' },
  { code: 'fin', name: 'Finnish' },
  { code: 'fra', name: 'French' },
  { code: 'glg', name: 'Galician' },
  { code: 'grc', name: 'Ancient Greek' },
  { code: 'guj', name: 'Gujarati' },
  { code: 'hat', name: 'Haitian' },
  { code: 'heb', name: 'Hebrew' },
  { code: 'hin', name: 'Hindi' },
  { code: 'hrv', name: 'Croatian' },
  { code: 'hun', name: 'Hungarian' },
  { code: 'hye', name: 'Armenian' },
  { code: 'ind', name: 'Indonesian' },
  { code: 'isl', name: 'Icelandic' },
  { code: 'ita', name: 'Italian' },
  { code: 'jpn', name: 'Japanese' },
  { code: 'kan', name: 'Kannada' },
  { code: 'kat', name: 'Georgian' },
  { code: 'kaz', name: 'Kazakh' },
  { code: 'khm', name: 'Khmer' },
  { code: 'kir', name: 'Kyrgyz' },
  { code: 'kor', name: 'Korean' },
  { code: 'lao', name: 'Lao' },
  { code: 'lat', name: 'Latin' },
  { code: 'lav', name: 'Latvian' },
  { code: 'lit', name: 'Lithuanian' },
  { code: 'mar', name: 'Marathi' },
  { code: 'mkd', name: 'Macedonian' },
  { code: 'mlt', name: 'Maltese' },
  { code: 'msa', name: 'Malay' },
  { code: 'mya', name: 'Burmese' },
  { code: 'nep', name: 'Nepali' },
  { code: 'nld', name: 'Dutch' },
  { code: 'nor', name: 'Norwegian' },
  { code: 'ori', name: 'Odia' },
  { code: 'pan', name: 'Punjabi' },
  { code: 'pol', name: 'Polish' },
  { code: 'por', name: 'Portuguese' },
  { code: 'pus', name: 'Pashto' },
  { code: 'ron', name: 'Romanian' },
  { code: 'rus', name: 'Russian' },
  { code: 'san', name: 'Sanskrit' },
  { code: 'sin', name: 'Sinhala' },
  { code: 'slk', name: 'Slovak' },
  { code: 'slv', name: 'Slovenian' },
  { code: 'spa', name: 'Spanish' },
  { code: 'sqi', name: 'Albanian' },
  { code: 'srp', name: 'Serbian' },
  { code: 'swa', name: 'Swahili' },
  { code: 'swe', name: 'Swedish' },
  { code: 'tam', name: 'Tamil' },
  { code: 'tel', name: 'Telugu' },
  { code: 'tgk', name: 'Tajik' },
  { code: 'tgl', name: 'Tagalog' },
  { code: 'tha', name: 'Thai' },
  { code: 'tir', name: 'Tigrinya' },
  { code: 'tur', name: 'Turkish' },
  { code: 'uig', name: 'Uyghur' },
  { code: 'ukr', name: 'Ukrainian' },
  { code: 'urd', name: 'Urdu' },
  { code: 'uzb', name: 'Uzbek' },
  { code: 'vie', name: 'Vietnamese' },
  { code: 'yid', name: 'Yiddish' },
  { code: 'yor', name: 'Yoruba' },
];

interface LanguageSelectProps {
  value: string[];
  onChange: (languages: string[]) => void;
}

export function LanguageSelect({ value, onChange }: LanguageSelectProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
        setSearch('');
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const selectedNames = useMemo(
    () =>
      value
        .map((code) => LANGUAGES.find((l) => l.code === code)?.name)
        .filter(Boolean) as string[],
    [value],
  );

  const filtered = useMemo(
    () =>
      search
        ? LANGUAGES.filter(
            (l) =>
              l.name.toLowerCase().includes(search.toLowerCase()) ||
              l.code.toLowerCase().includes(search.toLowerCase()),
          )
        : LANGUAGES,
    [search],
  );

  function toggle(code: string) {
    const next = value.includes(code)
      ? value.filter((c) => c !== code)
      : [...value, code];
    onChange(next);
  }

  return (
    <div ref={containerRef} className="relative w-full max-w-56">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={cn(
          'flex h-8 w-full items-center justify-between gap-1.5 rounded-lg border border-input bg-transparent px-2.5 py-1 text-sm transition-colors outline-none hover:bg-accent/50 focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50',
          open && 'border-ring',
        )}
      >
        <span className="truncate text-left text-muted-foreground">
          {selectedNames.length === 0
            ? 'Select languages'
            : `${selectedNames.length} selected`}
        </span>
        <ChevronDown
          className={cn(
            'size-4 shrink-0 text-muted-foreground transition-transform',
            open && 'rotate-180',
          )}
        />
      </button>

      {selectedNames.length > 0 && (
        <div className="mt-1 flex flex-wrap gap-1">
          {value.slice(0, 4).map((code) => {
            const lang = LANGUAGES.find((l) => l.code === code);
            if (!lang) return null;
            return (
              <span
                key={code}
                className="inline-flex items-center gap-0.5 rounded-md border bg-accent/30 px-1.5 py-0.5 text-xs"
              >
                {lang.name}
                <button
                  type="button"
                  onClick={() => toggle(code)}
                  className="ml-0.5 rounded-sm hover:bg-accent focus-visible:outline-none"
                >
                  <X className="size-3" />
                </button>
              </span>
            );
          })}
          {value.length > 4 && (
            <span className="text-xs text-muted-foreground">
              +{value.length - 4} more
            </span>
          )}
        </div>
      )}

      {open && (
        <div className="absolute left-0 right-0 top-full z-50 mt-1 max-h-72 overflow-hidden rounded-lg border bg-popover text-popover-foreground shadow-md ring-1 ring-foreground/10">
          <div className="relative flex items-center border-b px-2">
            <Search className="mr-1.5 size-4 shrink-0 text-muted-foreground" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search languages..."
              className="h-8 w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
          </div>
          <div className="overflow-y-auto max-h-56">
            {filtered.length === 0 ? (
              <div className="px-2.5 py-3 text-sm text-muted-foreground">
                No languages match
              </div>
            ) : (
              filtered.map((lang) => {
                const selected = value.includes(lang.code);
                return (
                  <button
                    key={lang.code}
                    type="button"
                    onClick={() => toggle(lang.code)}
                    className={cn(
                      'flex w-full items-center gap-2 px-2.5 py-1.5 text-sm text-left outline-none hover:bg-accent focus-visible:bg-accent',
                      selected && 'bg-accent/50',
                    )}
                  >
                    <span
                      className={cn(
                        'flex size-4 shrink-0 items-center justify-center rounded-sm border',
                        selected
                          ? 'border-primary bg-primary text-primary-foreground'
                          : 'border-input',
                      )}
                    >
                      {selected && <Check className="size-3" />}
                    </span>
                    <span className="flex-1 truncate">{lang.name}</span>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {lang.code}
                    </span>
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export { LANGUAGES };
