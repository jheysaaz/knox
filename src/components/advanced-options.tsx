import { useCallback, useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Card, CardContent } from "@/components/ui/card";
import {
  Scale,
  FileDown,
  Scan,
  SlidersHorizontal,
  HelpCircle,
} from "lucide-react";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectGroup,
  SelectItem,
} from "@/components/ui/select";

import { Switch } from "@/components/ui/switch";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";

type ProfileKey = "balanced" | "max-compression" | "high-fidelity";
type TabKey = ProfileKey | "custom";

/** Values for all advanced OCR settings configurable through the UI. */
export interface ProfileValues {
  cpuCores: number;
  memoryPages: number;
  binarization: "otsu" | "bradley-roth" | "fixed";
  fixedThreshold: number;
  deskew: "radon" | "hough" | "disabled";
  denoiseLevel: number;
  existingText: "skip" | "rasterize";
  psm: "auto" | "block" | "column" | "sparse";
  compression: "ccitt" | "flate";
  resolution: string;
  archiveEnforcement: boolean;
  languages: string;
}

const PROFILES: Record<ProfileKey, (cores: number) => ProfileValues> = {
  balanced: (c) => ({
    cpuCores: Math.max(1, c - 2),
    memoryPages: 30,
    binarization: "otsu",
    fixedThreshold: 128,
    deskew: "radon",
    denoiseLevel: 2,
    existingText: "skip",
    psm: "auto",
    compression: "ccitt",
    resolution: "300",
    archiveEnforcement: false,
    languages: "eng",
  }),
  "max-compression": (c) => ({
    cpuCores: c,
    memoryPages: 15,
    binarization: "otsu",
    fixedThreshold: 128,
    deskew: "radon",
    denoiseLevel: 4,
    existingText: "rasterize",
    psm: "auto",
    compression: "ccitt",
    resolution: "150",
    archiveEnforcement: false,
    languages: "eng",
  }),
  "high-fidelity": (c) => ({
    cpuCores: Math.max(1, Math.floor(c / 2)),
    memoryPages: 50,
    binarization: "fixed",
    fixedThreshold: 128,
    deskew: "radon",
    denoiseLevel: 0,
    existingText: "skip",
    psm: "auto",
    compression: "flate",
    resolution: "600",
    archiveEnforcement: true,
    languages: "eng",
  }),
};

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

/** Rendering of all advanced settings grouped by category. */
function SettingsPanel({
  values,
  onChange,
}: {
  values: ProfileValues;
  onChange: (patch: Partial<ProfileValues>) => void;
}) {
  return (
    <div className="space-y-5">
      {/* ── Hardware Allocation ── */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Hardware Allocation
        </h3>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-1.5 min-w-0">
              <span className="text-sm font-medium whitespace-nowrap">
                Thread Pool Capacity
              </span>
              <InfoTip>
                Limits the number of concurrent worker threads allocated to
                Rayon. Reserving at least 1 or 2 threads keeps the Host OS and
                Tauri UI smooth.
              </InfoTip>
            </div>
            <span className="text-sm tabular-nums text-muted-foreground shrink-0 w-6 text-right">
              {values.cpuCores}
            </span>
          </div>
          <Slider
            min={1}
            max={16}
            step={1}
            value={[values.cpuCores]}
            onValueChange={([v]) => onChange({ cpuCores: v })}
          />
        </div>

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
      </div>

      {/* ── Image Pre-processing ── */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Image Pre-processing
        </h3>

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
            onValueChange={(v) => onChange({ binarization: v as "otsu" | "bradley-roth" | "fixed" })}
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

        {values.binarization === "fixed" && (
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
            onValueChange={(v) => onChange({ deskew: v as "radon" | "hough" | "disabled" })}
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
      </div>

      {/* ── OCR Recognition Engine ── */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          OCR Recognition Engine
        </h3>

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
            onValueChange={(v) => onChange({ existingText: v as "skip" | "rasterize" })}
          >
            <RadioGroupItem value="skip">
              <div className="flex flex-col">
                <span className="text-sm font-medium">
                  Skip Native Text Pages
                </span>
              </div>
            </RadioGroupItem>
            <RadioGroupItem value="rasterize">
              <div className="flex flex-col">
                <span className="text-sm font-medium">
                  Force Rasterize &amp; Overwrite
                </span>
              </div>
            </RadioGroupItem>
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
            onValueChange={(v) => onChange({ psm: v as "auto" | "block" | "column" | "sparse" })}
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

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-sm font-medium whitespace-nowrap">Languages</span>
            <InfoTip>
              Tesseract language packs to load, separated by + (e.g. eng+spa).
            </InfoTip>
          </div>
          <Input
            value={values.languages}
            onChange={(e) => onChange({ languages: e.target.value })}
            className="w-40"
            placeholder="eng+spa"
          />
        </div>
      </div>

      {/* ── Compression & Standards ── */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Compression &amp; Standards
        </h3>

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
            onValueChange={(v) => onChange({ compression: v as "ccitt" | "flate" })}
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
      </div>
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
  const cores = useMemo(() => navigator.hardwareConcurrency || 8, []);

  const getProfile = useCallback(
    (key: ProfileKey): ProfileValues => PROFILES[key](cores),
    [cores],
  );

  const activeTab = useMemo<TabKey>(() => {
    if (customOverride) return "custom";
    for (const key of Object.keys(PROFILES) as ProfileKey[]) {
      const profile = getProfile(key);
      const matches = (Object.keys(profile) as (keyof ProfileValues)[]).every(
        (field) => profile[field] === value[field],
      );
      if (matches) return key;
    }
    return "custom";
  }, [getProfile, value, customOverride]);

  const handleTabChange = useCallback(
    (tab: string) => {
      const key = tab as TabKey;
      if (key === "custom") {
        setCustomOverride(true);
        return;
      }
      setCustomOverride(false);
      onChange(getProfile(key));
    },
    [getProfile, onChange],
  );



  const handleChange = useCallback(
    (patch: Partial<ProfileValues>) => {
      onChange({ ...value, ...patch });
    },
    [onChange, value],
  );

  const tabs = useMemo(
    () => [
      { key: "balanced" as TabKey, icon: Scale, label: "Balanced" },
      {
        key: "max-compression" as TabKey,
        icon: FileDown,
        label: "Max Compression",
      },
      { key: "high-fidelity" as TabKey, icon: Scan, label: "High Fidelity" },
      { key: "custom" as TabKey, icon: SlidersHorizontal, label: "Custom" },
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
            <CardContent className="pt-4 pb-4">
              <SettingsPanel values={value} onChange={handleChange} />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
