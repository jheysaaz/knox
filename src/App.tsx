import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { check } from '@tauri-apps/plugin-updater';
import { Download } from 'lucide-react';
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { Toaster, toast } from 'sonner';
import type { ProfileValues } from '@/components/advanced-options';
import { Header } from '@/components/header';
import { PasswordDialog } from '@/components/password-dialog';
import { Progress } from '@/components/ui/progress';
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

  const checkForUpdates = useCallback(async () => {
    try {
      const [currentVersion, update] = await Promise.all([
        getVersion(),
        check(),
      ]);
      if (!update) return;

      const currentPre = currentVersion.includes('-')
        ? currentVersion.split('-')[1].split('.')[0]
        : null;
      const updatePre = update.version.includes('-')
        ? update.version.split('-')[1].split('.')[0]
        : null;
      if (currentPre !== updatePre) return;

      const maybeNotify = async (title: string, body?: string) => {
        try {
          const granted = await isPermissionGranted();
          const ok = granted || (await requestPermission()) === 'granted';
          if (ok) sendNotification({ title, body });
        } catch {
          /* silent */
        }
      };

      const toastId = toast(
        `Update v${update.version} available`,
        {
          description: update.body || undefined,
          duration: 20_000,
          action: {
            label: 'Download',
            onClick: async () => {
              let downloaded = 0;
              let contentLength = 0;
              toast.loading('Downloading update...', { id: toastId });
              try {
                await update.downloadAndInstall((event) => {
                  if (event.event === 'Started') {
                    contentLength = event.data.contentLength ?? 0;
                  } else if (event.event === 'Progress') {
                    downloaded += event.data.chunkLength;
                    const pct = contentLength
                      ? Math.round((downloaded / contentLength) * 100)
                      : 0;
                    toast(
                      <div className="text-sm">
                        <Progress value={pct} className="mt-2" />
                        <p className="text-xs !text-muted-foreground mt-1">
                          {pct}%
                        </p>
                      </div>,
                      { id: toastId, duration: Infinity },
                    );
                  }
                });
                toast.success('Update ready! Restart to apply.', {
                  id: toastId,
                  duration: 30_000,
                  action: {
                    label: 'Restart now',
                    onClick: () => invoke('restart_app'),
                  },
                });
                maybeNotify(
                  'Update ready',
                  `v${update.version} downloaded. Restart to apply.`,
                );
              } catch {
                toast.error('Update download failed', { id: toastId });
              }
            },
          },
        },
      );
      maybeNotify(
        'Update available',
        `v${update.version} is ready to download.`,
      );
    } catch {
      // silent — update check is best-effort
    }
  }, []);

  const checkedRef = useRef(false);

  useEffect(() => {
    if (checkedRef.current) return;
    checkedRef.current = true;
    checkForUpdates();
  }, [checkForUpdates]);

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
      {import.meta.env.DEV && (
        <button
          onClick={checkForUpdates}
          className="fixed bottom-4 left-4 z-50 flex items-center gap-1.5 rounded-full bg-muted/80 px-3 py-1.5 text-xs text-muted-foreground backdrop-blur-sm transition-colors hover:bg-muted hover:text-foreground"
          title="Check for updates (dev)"
        >
          <Download size={14} />
          Check Update
        </button>
      )}
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
              '!p-4 !gap-3 !items-center !rounded-xl !shadow-lg !border backdrop-blur-md !bg-background !text-foreground',
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
