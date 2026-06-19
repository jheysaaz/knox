import { lazy, Suspense, useState } from 'react';
import { Toaster } from 'sonner';
import type { ProfileValues } from '@/components/advanced-options';
import { Header } from '@/components/header';
import { PasswordDialog } from '@/components/password-dialog';
import { Spinner } from '@/components/ui/spinner';
import { TooltipProvider } from '@/components/ui/tooltip';
import { GREETING } from '@/hooks/useGreeting';
import { useLogger } from '@/hooks/useLogger';
import { useQueue } from '@/hooks/useQueue';

const LeftPanel = lazy(() => import('./components/left-panel'));
const RightPanel = lazy(() => import('./components/right-panel'));

/** macOS overlay title bar safe zone — only needed when titleBarStyle: "Overlay" (macOS-only). */
const SAFE_ZONE_TOP = navigator.userAgent.includes('Mac') ? 38 : 0;

export default function App() {
  const { logs, addLog } = useLogger();
  const {
    files,
    outputDir,
    setOutputDir,
    isRunning,
    starting,
    history,
    handleFilesAdded,
    handleFileRemove,
    handleFileReprocess,
    handleClearFiles,
    handleStart,
    handleStop,
    handleClearHistory,
    passwordDialogOpen,
    pendingEncryptedFiles,
    handlePasswordConfirm,
    handlePasswordCancel,
  } = useQueue(addLog);
  const [settings, setSettings] = useState<ProfileValues>({
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
  });
  const [activeTab, setActiveTab] = useState('queue');

  return (
    <>
      <TooltipProvider>
        <div
          className="flex gap-6 lg:gap-8 xl:gap-12 px-6 lg:px-10 xl:px-16 pb-6 h-dvh overflow-hidden max-w-[1600px] mx-auto"
          style={{ paddingTop: SAFE_ZONE_TOP > 0 ? SAFE_ZONE_TOP : undefined }}
        >
          <div className="flex-[3] min-w-0 flex flex-col min-h-0">
            <div className="flex-1 min-h-0 overflow-y-auto p-[3px] -m-[3px]">
              <Header greeting={GREETING} />
              <Suspense fallback={<Spinner />}>
                <LeftPanel
                  onFilesAdded={handleFilesAdded}
                  outputDir={outputDir}
                  onOutputDirChange={setOutputDir}
                  settings={settings}
                  onSettingsChange={setSettings}
                  isRunning={isRunning}
                  starting={starting}
                  onStart={handleStart}
                />
              </Suspense>
            </div>
          </div>

          <div className="hidden lg:block w-px bg-border self-stretch shrink-0" />

          <div className="flex-[2] min-w-0 flex flex-col gap-2">
            <Suspense fallback={<Spinner />}>
              <RightPanel
                files={files}
                onFileRemove={handleFileRemove}
                onClear={handleClearFiles}
                onReprocess={handleFileReprocess}
                isRunning={isRunning}
                onStop={handleStop}
                logs={logs}
                history={history}
                onClearHistory={handleClearHistory}
                activeTab={activeTab}
                onTabChange={setActiveTab}
              />
            </Suspense>
          </div>
        </div>
      </TooltipProvider>
      <PasswordDialog
        open={passwordDialogOpen}
        fileNames={pendingEncryptedFiles.map((f) => f.name)}
        onConfirm={handlePasswordConfirm}
        onCancel={handlePasswordCancel}
      />
      <Toaster
        position="bottom-right"
        duration={5000}
        toastOptions={{
          classNames: {
            toast:
              '!p-4 !gap-3 !items-center !rounded-xl !shadow-lg !border backdrop-blur-md',
            error:
              '!bg-red-50 dark:!bg-red-950/30 !border-red-600 dark:!border-red-500 !text-red-700 dark:!text-red-400',
            success:
              '!bg-emerald-50 dark:!bg-emerald-950/30 !border-emerald-600 dark:!border-emerald-500 !text-emerald-700 dark:!text-emerald-400',
            warning:
              '!bg-amber-50 dark:!bg-amber-950/30 !border-amber-600 dark:!border-amber-500 !text-amber-700 dark:!text-amber-400',
            info: '!bg-blue-50 dark:!bg-blue-950/30 !border-blue-600 dark:!border-blue-500 !text-blue-700 dark:!text-blue-400',
          },
        }}
      />
    </>
  );
}
