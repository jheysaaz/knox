import {
  ChevronRight,
  FileDown,
  HelpCircle,
  RotateCcw,
  Save,
  Scale,
  Scan,
  SlidersHorizontal,
  Trash2,
} from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { LanguageSelect } from '@/components/language-select';
import { Card, CardContent } from '@/components/ui/card';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';

type ProfileKey = 'balanced' | 'max-compression' | 'high-fidelity';
type TabKey = ProfileKey | 'custom';

/** Values for all advanced OCR settings configurable through the UI. */
export interface ProfileValues {
  memoryPages: number;
  binarization: 'otsu' | 'bradley-roth' | 'fixed';
  fixedThreshold: number;
  deskew: 'radon' | 'hough' | 'disabled';
  denoiseLevel: number;
  existingText: 'skip' | 'rasterize';
  psm: 'auto' | 'block' | 'column' | 'sparse';
  compression: 'ccitt' | 'flate';
  resolution: string;
  archiveEnforcement: boolean;
  languages: string[];
  safeMode: boolean;
  continueOnError: boolean;
}

const PROFILES: Record<ProfileKey, ProfileValues> = {
  balanced: {
    memoryPages: 30,
    binarization: 'otsu',
    fixedThreshold: 128,
    deskew: 'radon',
    denoiseLevel: 2,
    existingText: 'skip',
    psm: 'auto',
    compression: 'flate',
    resolution: '300',
    archiveEnforcement: false,
    languages: ['eng', 'spa'],
    safeMode: false,
    continueOnError: false,
  },
  'max-compression': {
    memoryPages: 15,
    binarization: 'otsu',
    fixedThreshold: 128,
    deskew: 'radon',
    denoiseLevel: 4,
    existingText: 'rasterize',
    psm: 'auto',
    compression: 'ccitt',
    resolution: '150',
    archiveEnforcement: false,
    languages: ['eng', 'spa'],
    safeMode: false,
    continueOnError: false,
  },
  'high-fidelity': {
    memoryPages: 50,
    binarization: 'fixed',
    fixedThreshold: 128,
    deskew: 'radon',
    denoiseLevel: 0,
    existingText: 'skip',
    psm: 'auto',
    compression: 'flate',
    resolution: '600',
    archiveEnforcement: true,
    languages: ['eng', 'spa'],
    safeMode: false,
    continueOnError: false,
  },
};

const STORAGE_KEY = 'knox-custom-profiles';

interface SavedProfile {
  name: string;
  values: ProfileValues;
}

function loadSavedProfiles(): SavedProfile[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function persistSavedProfiles(profiles: SavedProfile[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles));
}

/** Small tooltip icon that shows a help text on hover. */
function InfoTip({ children }: { children: React.ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          className="inline-flex items-center cursor-help text-muted-foreground hover:text-foreground transition-colors"
        >
          <HelpCircle className="h-3.5 w-3.5" />
        </button>
      </TooltipTrigger>
      <TooltipContent side="right" className="max-w-64">
        {children}
      </TooltipContent>
    </Tooltip>
  );
}

const SECTION_FIELDS = {
  hardware: [
    'memoryPages',
    'safeMode',
    'continueOnError',
  ] as (keyof ProfileValues)[],
  image: [
    'binarization',
    'fixedThreshold',
    'deskew',
    'denoiseLevel',
  ] as (keyof ProfileValues)[],
  ocr: ['existingText', 'psm', 'languages'] as (keyof ProfileValues)[],
  compression: [
    'compression',
    'resolution',
    'archiveEnforcement',
  ] as (keyof ProfileValues)[],
};

function Section({
  title,
  fields,
  onChange,
  children,
}: {
  title: string;
  fields: (keyof ProfileValues)[];
  onChange: (patch: Partial<ProfileValues>) => void;
  children: React.ReactNode;
}) {
  const handleReset = () => {
    const patch: Record<string, unknown> = {};
    for (const field of fields) {
      const val = PROFILES.balanced[field];
      patch[field] = Array.isArray(val) ? [...val] : val;
    }
    onChange(patch as Partial<ProfileValues>);
  };

  return (
    <details className="group space-y-0">
      <summary className="flex items-center gap-2 cursor-pointer list-none text-xs font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground transition-colors py-1.5 rounded select-none">
        <ChevronRight className="h-3.5 w-3.5 transition-transform group-open:rotate-90 shrink-0" />
        <span>{title}</span>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            handleReset();
          }}
          className="ml-auto inline-flex items-center gap-1 text-xs font-normal normal-case text-muted-foreground hover:text-foreground transition-colors opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          title="Reset section to Balanced defaults"
        >
          <RotateCcw className="h-3 w-3" />
          Reset
        </button>
      </summary>
      <div className="space-y-3 pt-2 pb-1">{children}</div>
    </details>
  );
}

/** Rendering of all advanced settings grouped by category. */
function SettingsPanel({
  values,
  onChange,
}: {
  values: ProfileValues;
  onChange: (patch: Partial<ProfileValues>) => void;
}) {
  return (
    <div className="space-y-1">
      {/* ── Hardware Allocation ── */}
      <Section
        title="Hardware Allocation"
        fields={SECTION_FIELDS.hardware}
        onChange={onChange}
      >
        <div className="space-y-3">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-1.5 min-w-0">
              <span className="text-sm font-medium whitespace-nowrap">
                In-Memory Page Cap
              </span>
              <InfoTip>
                The maximum number of raw page bitmaps allowed in RAM
                concurrently via our async Semaphore. Lower values prevent
                out-of-memory crashes on low-spec hardware.
              </InfoTip>
            </div>
            <span className="text-sm tabular-nums text-muted-foreground shrink-0 w-8 text-right">
              {values.memoryPages}
            </span>
          </div>
          <Slider
            min={5}
            max={100}
            step={5}
            value={[values.memoryPages]}
            onValueChange={([v]) => onChange({ memoryPages: v })}
          />
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Safe Mode (1 file at a time)
            </span>
            <InfoTip>
              Forces the engine to process only one file at a time, reducing
              memory pressure on low-spec machines. Useful when processing very
              large PDFs or running on machines with limited RAM.
            </InfoTip>
          </div>
          <Switch
            checked={values.safeMode}
            onCheckedChange={(v) => onChange({ safeMode: v })}
          />
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Skip corrupt pages
            </span>
            <InfoTip>
              When enabled, pages that fail during processing (e.g. corrupt
              images, OCR errors) are skipped instead of failing the entire
              file.
            </InfoTip>
          </div>
          <Switch
            checked={values.continueOnError}
            onCheckedChange={(v) => onChange({ continueOnError: v })}
          />
        </div>
      </Section>

      {/* ── Image Pre-processing ── */}
      <Section
        title="Image Pre-processing"
        fields={SECTION_FIELDS.image}
        onChange={onChange}
      >
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Image Thresholding Mode
            </span>
            <InfoTip>
              Determines how the engine separates background noise from text.
              Otsu computes the optimal contrast per page; Fixed uses a static
              grayscale cut-off.
            </InfoTip>
          </div>
          <Select
            value={values.binarization}
            onValueChange={(v) =>
              onChange({ binarization: v as 'otsu' | 'bradley-roth' | 'fixed' })
            }
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="otsu">Otsu Adaptive</SelectItem>
                <SelectItem value="bradley-roth">Bradley-Roth</SelectItem>
                <SelectItem value="fixed">Fixed Threshold</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        {values.binarization === 'fixed' && (
          <div className="space-y-3">
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-1.5 min-w-0">
                <span className="text-sm font-medium whitespace-nowrap">
                  Fixed Threshold
                </span>
                <InfoTip>
                  Uses a fixed grayscale cutoff (0-255). Lower values preserve
                  faint text; higher values remove background noise.
                </InfoTip>
              </div>
              <span className="text-sm tabular-nums text-muted-foreground shrink-0 w-10 text-right">
                {values.fixedThreshold}
              </span>
            </div>
            <Slider
              min={0}
              max={255}
              step={1}
              value={[values.fixedThreshold]}
              onValueChange={([v]) => onChange({ fixedThreshold: v })}
            />
          </div>
        )}

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Orientation Alignment
            </span>
            <InfoTip>
              Algorithm used to detect and rotate crooked scans. Radon transform
              yields superior global angle detection for heavily degraded or
              noisy pages.
            </InfoTip>
          </div>
          <Select
            value={values.deskew}
            onValueChange={(v) =>
              onChange({ deskew: v as 'radon' | 'hough' | 'disabled' })
            }
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="radon">Radon Transform</SelectItem>
                <SelectItem value="hough">Hough Line Transform</SelectItem>
                <SelectItem value="disabled">Disabled</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-1.5 min-w-0">
              <span className="text-sm font-medium whitespace-nowrap">
                Despeckle Intensity
              </span>
              <InfoTip>
                Applies morphological opening/closing to erase scanner spots.
                Warning: Values higher than 3 may inadvertently erase small
                punctuation like periods, commas, or dot-accents.
              </InfoTip>
            </div>
            <span className="text-sm tabular-nums text-muted-foreground shrink-0 w-6 text-right">
              {values.denoiseLevel}
            </span>
          </div>
          <Slider
            min={0}
            max={5}
            step={1}
            value={[values.denoiseLevel]}
            onValueChange={([v]) => onChange({ denoiseLevel: v })}
          />
        </div>
      </Section>

      {/* ── OCR Recognition Engine ── */}
      <Section
        title="OCR Recognition Engine"
        fields={SECTION_FIELDS.ocr}
        onChange={onChange}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-1.5">
            <span className="text-sm font-medium">Ingestion Override</span>
            <InfoTip>
              Skip bypasses OCR on hybrid pages that already contain
              high-quality vector text streams. Force Rasterize converts
              everything to bitmaps first, fixing corrupted or hidden font
              encodings.
            </InfoTip>
          </div>
          <RadioGroup
            value={values.existingText}
            onValueChange={(v) =>
              onChange({ existingText: v as 'skip' | 'rasterize' })
            }
          >
            <div className="flex items-center gap-2">
              <RadioGroupItem value="skip" id="existing-skip" />
              <label
                htmlFor="existing-skip"
                className="text-sm font-medium cursor-pointer"
              >
                Skip Native Text Pages
              </label>
            </div>
            <div className="flex items-center gap-2">
              <RadioGroupItem value="rasterize" id="existing-rasterize" />
              <label
                htmlFor="existing-rasterize"
                className="text-sm font-medium cursor-pointer"
              >
                Force Rasterize &amp; Overwrite
              </label>
            </div>
          </RadioGroup>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Tesseract Page Layout (PSM)
            </span>
            <InfoTip>
              Tells the underlying OCR engine how to parse layout geometry.
              Single Column is optimized for receipts, logs, and tables;
              Automatic handles standard multi-column layouts.
            </InfoTip>
          </div>
          <Select
            value={values.psm}
            onValueChange={(v) =>
              onChange({ psm: v as 'auto' | 'block' | 'column' | 'sparse' })
            }
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="auto">Fully Automatic</SelectItem>
                <SelectItem value="block">Single Uniform Block</SelectItem>
                <SelectItem value="column">Single Column text</SelectItem>
                <SelectItem value="sparse">Sparse Text</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0 pt-0.5">
            <span className="text-sm font-medium whitespace-nowrap">
              Languages
            </span>
            <InfoTip>
              Tesseract language packs to load. Missing packs are automatically
              downloaded before processing starts.
            </InfoTip>
          </div>
          <LanguageSelect
            value={values.languages}
            onChange={(v) => onChange({ languages: v })}
          />
        </div>
      </Section>

      {/* ── Compression & Standards ── */}
      <Section
        title="Compression & Standards"
        fields={SECTION_FIELDS.compression}
        onChange={onChange}
      >
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Bi-level Stream Compression
            </span>
            <InfoTip>
              CCITT Group 4 (Fax encoding) is the absolute gold standard for
              1-bit monochrome text. FlateDecode will automatically be used if a
              page contains preserved color channels.
            </InfoTip>
          </div>
          <Select
            value={values.compression}
            onValueChange={(v) =>
              onChange({ compression: v as 'ccitt' | 'flate' })
            }
          >
            <SelectTrigger className="w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="ccitt">CCITT Group 4</SelectItem>
                <SelectItem value="flate">FlateDecode + Predictor</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Output Image Downsampling
            </span>
            <InfoTip>
              Downsample heavy source graphics to a fixed DPI before writing
              back to the PDF catalog. 300 DPI is the standard industry
              equilibrium for document readability vs file size.
            </InfoTip>
          </div>
          <Select
            value={values.resolution}
            onValueChange={(v) => onChange({ resolution: v })}
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="150">150 DPI (Web)</SelectItem>
                <SelectItem value="300">300 DPI (Standard Print)</SelectItem>
                <SelectItem value="600">600 DPI (Archival)</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">
              Enforce PDF/A-2b Compliance
            </span>
            <InfoTip>
              Forces the engine to output structural metadata streams and embed
              mandatory color intent data, fulfilling international legal
              preservation standards for archival auditing.
            </InfoTip>
          </div>
          <Switch
            checked={values.archiveEnforcement}
            onCheckedChange={(v) => onChange({ archiveEnforcement: v })}
          />
        </div>
      </Section>
    </div>
  );
}

/** Profile selector (Balanced / Max Compression / High Fidelity / Custom) with access
 * to the full settings panel when in Custom mode. */
export function AdvancedOptions({
  value,
  onChange,
}: {
  value: ProfileValues;
  onChange: (next: ProfileValues) => void;
}) {
  const [customOverride, setCustomOverride] = useState(false);
  const [saveName, setSaveName] = useState('');
  const [profileVersion, setProfileVersion] = useState(0);

  const savedProfiles = useMemo(() => loadSavedProfiles(), [profileVersion]);

  const activeTab = useMemo<TabKey>(() => {
    if (customOverride) return 'custom';
    for (const key of Object.keys(PROFILES) as ProfileKey[]) {
      const profile = PROFILES[key];
      const matches = (Object.keys(profile) as (keyof ProfileValues)[]).every(
        (field) => {
          const a = profile[field];
          const b = value[field];
          if (Array.isArray(a) && Array.isArray(b)) {
            return a.length === b.length && a.every((v, i) => v === b[i]);
          }
          return a === b;
        },
      );
      if (matches) return key;
    }
    return 'custom';
  }, [value, customOverride]);

  const handleTabChange = useCallback(
    (tab: string) => {
      const key = tab as TabKey;
      if (key === 'custom') {
        setCustomOverride(true);
        return;
      }
      setCustomOverride(false);
      onChange(PROFILES[key]);
    },
    [onChange],
  );

  const handleChange = useCallback(
    (patch: Partial<ProfileValues>) => {
      onChange({ ...value, ...patch });
    },
    [onChange, value],
  );

  const handleSaveProfile = useCallback(() => {
    const name = saveName.trim();
    if (!name) return;
    const profiles = loadSavedProfiles();
    const idx = profiles.findIndex((p) => p.name === name);
    const entry: SavedProfile = {
      name,
      values: { ...value, languages: [...value.languages] },
    };
    if (idx >= 0) profiles[idx] = entry;
    else profiles.push(entry);
    persistSavedProfiles(profiles);
    setSaveName('');
    setProfileVersion((v) => v + 1);
  }, [saveName, value]);

  const handleDeleteProfile = useCallback((name: string) => {
    const profiles = loadSavedProfiles().filter((p) => p.name !== name);
    persistSavedProfiles(profiles);
    setProfileVersion((v) => v + 1);
  }, []);

  const handleLoadProfile = useCallback(
    (profile: SavedProfile) => {
      setCustomOverride(true);
      onChange(profile.values);
    },
    [onChange],
  );

  const tabs = useMemo(
    () => [
      { key: 'balanced' as TabKey, icon: Scale, label: 'Balanced' },
      {
        key: 'max-compression' as TabKey,
        icon: FileDown,
        label: 'Max Compression',
      },
      { key: 'high-fidelity' as TabKey, icon: Scan, label: 'High Fidelity' },
      { key: 'custom' as TabKey, icon: SlidersHorizontal, label: 'Custom' },
    ],
    [],
  );

  return (
    <div className="space-y-2">
      <label className="text-sm font-medium text-foreground">Profile</label>
      <Tabs value={activeTab} onValueChange={handleTabChange}>
        <TabsList className="mx-auto">
          {tabs.map(({ key, icon: Icon, label }) => (
            <TabsTrigger key={key} value={key}>
              <Icon className="h-4 w-4" />
              {label}
            </TabsTrigger>
          ))}
        </TabsList>

        <TabsContent value="custom">
          <Card className="w-full ring-inset">
            <CardContent className="pt-4 pb-4 space-y-4">
              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={saveName}
                  onChange={(e) => setSaveName(e.target.value)}
                  placeholder="Profile name…"
                  className="flex-1 h-8 px-2 text-sm rounded-md border border-input bg-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSaveProfile();
                  }}
                />
                <button
                  type="button"
                  onClick={handleSaveProfile}
                  disabled={!saveName.trim()}
                  className="inline-flex items-center gap-1.5 h-8 px-3 text-sm font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:pointer-events-none"
                >
                  <Save className="h-3.5 w-3.5" />
                  Save
                </button>
              </div>

              {savedProfiles.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {savedProfiles.map((profile) => (
                    <div
                      key={profile.name}
                      className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/50 px-2.5 py-1 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors group"
                    >
                      <button
                        type="button"
                        onClick={() => handleLoadProfile(profile)}
                        className="hover:text-foreground transition-colors"
                      >
                        {profile.name}
                      </button>
                      <button
                        type="button"
                        onClick={() => handleDeleteProfile(profile.name)}
                        className="opacity-0 group-hover:opacity-100 hover:text-destructive transition-all"
                        title="Delete profile"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}

              <SettingsPanel values={value} onChange={handleChange} />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
