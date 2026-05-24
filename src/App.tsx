import { Suspense, lazy, useState } from "react";
import { Toaster } from "sonner";
import { Header } from "@/components/header";
import { Spinner } from "@/components/ui/spinner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { GREETING } from "@/hooks/useGreeting";
import { useLogger } from "@/hooks/useLogger";
import { useQueue } from "@/hooks/useQueue";
import type { ProfileValues } from "@/components/advanced-options";

const LeftPanel = lazy(() => import("./components/left-panel"));
const RightPanel = lazy(() => import("./components/right-panel"));

export default function App() {
  const { logs, addLog } = useLogger();
  const {
    files,
    outputDir,
    setOutputDir,
    showActivity,
    setShowActivity,
    isRunning,
    handleFilesAdded,
    handleFileRemove,
    handleFileReprocess,
    handleClearFiles,
    handleStart,
    handleStop,
  } = useQueue(addLog);
  const [settings, setSettings] = useState<ProfileValues>({
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
    languages: ["eng", "spa"],
  });

  return (
    <>
      <TooltipProvider>
        <div className="flex gap-6 pt-10 pr-6 pb-6 pl-6 h-dvh overflow-hidden">
          <div className="flex-[3] min-w-0 flex flex-col min-h-0">
            <div className="flex-1 min-h-0 overflow-y-auto">
              <Header
                greeting={GREETING}
                showActivity={showActivity}
                onToggleActivity={() => setShowActivity((v) => !v)}
              />
              <Suspense fallback={<Spinner />}>
                <LeftPanel
                  onFilesAdded={handleFilesAdded}
                  outputDir={outputDir}
                  onOutputDirChange={setOutputDir}
                  settings={settings}
                  onSettingsChange={setSettings}
                  isRunning={isRunning}
                  onStart={handleStart}
                />
              </Suspense>
            </div>
          </div>

          <div className="flex-[2] min-w-0 flex flex-col gap-2">
            <Suspense fallback={<Spinner />}>
              <RightPanel
                files={files}
                onFileRemove={handleFileRemove}
                onClear={handleClearFiles}
                onReprocess={handleFileReprocess}
                isRunning={isRunning}
                onStop={handleStop}
                showActivity={showActivity}
                logs={logs}
              />
            </Suspense>
          </div>
        </div>
      </TooltipProvider>
      <Toaster
        position="top-right"
        duration={5000}
        toastOptions={{
          classNames: {
            toast:
              "!p-4 !gap-3 !items-center !rounded-xl !shadow-lg !border backdrop-blur-md",
            error:
              "!bg-red-50 dark:!bg-red-950/30 !border-red-600 dark:!border-red-500 !text-red-700 dark:!text-red-400",
            success:
              "!bg-emerald-50 dark:!bg-emerald-950/30 !border-emerald-600 dark:!border-emerald-500 !text-emerald-700 dark:!text-emerald-400",
            warning:
              "!bg-amber-50 dark:!bg-amber-950/30 !border-amber-600 dark:!border-amber-500 !text-amber-700 dark:!text-amber-400",
            info: "!bg-blue-50 dark:!bg-blue-950/30 !border-blue-600 dark:!border-blue-500 !text-blue-700 dark:!text-blue-400",
          },
        }}
      />
    </>
  );
}
