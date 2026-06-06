import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sidebar } from "./components/Sidebar";
import { DashboardTab } from "./components/DashboardTab";
import { RepositoryTab } from "./components/RepositoryTab";
import { PlaybooksTab } from "./components/PlaybooksTab";
import { OptionsTab } from "./components/OptionsTab";
import { LogsTab } from "./components/LogsTab";
import { StartPlaybookModal } from "./components/StartPlaybookModal";
import { DaemonStatus, CacheStatus, Playbook, LogEntry } from "./types";

export function App() {
  const [activeTab, setActiveTab] = useState<"dashboard" | "repository" | "playbooks" | "options" | "logs">("dashboard");
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus>({ status: "offline" });
  const [cacheStatus, setCacheStatus] = useState<CacheStatus | null>(null);
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([
    {
      time: new Date().toLocaleTimeString(),
      message: "Console initialized. Ready to interact with pyro-daemond.",
      type: "system",
    },
  ]);
  const [startModalOpen, setStartModalOpen] = useState(false);
  const [selectedPlaybookName, setSelectedPlaybookName] = useState<string | null>(null);
  const [errorNotification, setErrorNotification] = useState<{ title: string; message: string } | null>(null);

  const showError = useCallback((title: string, message: string) => {
    setErrorNotification({ title, message });
  }, []);

  useEffect(() => {
    if (errorNotification) {
      const timer = setTimeout(() => {
        setErrorNotification(null);
      }, 7000);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [errorNotification]);

  // Helper to add log entries
  const addLog = useCallback((message: string, type: LogEntry["type"] = "system") => {
    setLogs((prev) => [
      ...prev,
      {
        time: new Date().toLocaleTimeString(),
        message,
        type,
      },
    ]);
  }, []);

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  // 1. Query Daemon Status
  const queryDaemonStatus = useCallback(async () => {
    try {
      addLog("Querying daemon status...", "command");
      const res = (await invoke("get_daemon_status")) as DaemonStatus;

      setDaemonStatus(res);
      if (res.status === "online") {
        addLog(
          `Daemon is online. Version ${res.version}. Active workers: ${res.active_workers}.`,
          "success"
        );
      } else {
        addLog(`Daemon offline. ${res.message || ""}`, "error");
      }
    } catch (err) {
      setDaemonStatus({ status: "error", message: String(err) });
      addLog(`Failed to query daemon: ${err}`, "error");
      showError("Daemon Connection Error", `Failed to query daemon: ${err}`);
    }
  }, [addLog, showError]);

  // 2. Load Cache Status
  const loadCache = useCallback(async () => {
    try {
      addLog("Fetching local cache repository status...", "command");
      const res = (await invoke("get_cache_status")) as CacheStatus;
      setCacheStatus(res);
      addLog(
        `Loaded ${res.capabilities.length} capabilities and ${res.playbooks.length} playbooks from cache.`,
        "success"
      );
    } catch (err) {
      addLog(`Failed to load cache info: ${err}`, "error");
      showError("Cache Query Failed", String(err));
    }
  }, [addLog, showError]);

  // 3. Load Playbooks
  const loadPlaybooks = useCallback(async () => {
    try {
      addLog("Listing active playbooks from daemon...", "command");
      const res = (await invoke("list_active_playbooks")) as Playbook[];
      setPlaybooks(res);
      addLog(`Retrieved ${res.length} active playbook workers.`, "success");
    } catch (err) {
      addLog(`Failed to list playbooks: ${err}`, "error");
      showError("Failed to List Playbooks", String(err));
    }
  }, [addLog, showError]);

  // 4. Purge Cache
  const purgeCache = useCallback(async () => {
    try {
      addLog("Purging cache...", "command");
      const msg = (await invoke("purge_cache")) as string;
      addLog(msg, "success");
      await loadCache();
    } catch (err) {
      addLog(`Failed to purge cache: ${err}`, "error");
      showError("Purge Cache Failed", String(err));
      throw err;
    }
  }, [addLog, loadCache, showError]);

  const purgeCapabilities = useCallback(async () => {
    try {
      addLog("Purging capabilities cache...", "command");
      const msg = (await invoke("purge_capabilities_cache")) as string;
      addLog(msg, "success");
      await loadCache();
    } catch (err) {
      addLog(`Failed to purge capabilities: ${err}`, "error");
      showError("Purge Capabilities Failed", String(err));
      throw err;
    }
  }, [addLog, loadCache, showError]);

  const purgePlaybooks = useCallback(async () => {
    try {
      addLog("Purging playbooks cache...", "command");
      const msg = (await invoke("purge_playbooks_cache")) as string;
      addLog(msg, "success");
      await loadCache();
    } catch (err) {
      addLog(`Failed to purge playbooks: ${err}`, "error");
      showError("Purge Playbooks Failed", String(err));
      throw err;
    }
  }, [addLog, loadCache, showError]);

  // 5. Stop Playbook
  const stopPlaybook = useCallback(
    async (name: string) => {
      const confirmed = window.confirm(`Are you sure you want to stop playbook worker "${name}"?`);
      if (!confirmed) return;

      try {
        addLog(`Requesting stop for playbook "${name}"...`, "command");
        const msg = (await invoke("stop_playbook", { name })) as string;
        addLog(msg, "success");

        await invoke("delete_playbook", { name });
        addLog(`Deleted playbook worker state mapping for "${name}"`, "system");

        await loadPlaybooks();
        await queryDaemonStatus();
      } catch (err) {
        addLog(`Failed to stop playbook: ${err}`, "error");
        showError("Failed to Stop Playbook", String(err));
      }
    },
    [addLog, loadPlaybooks, queryDaemonStatus, showError]
  );

  // 6. Start Playbook
  const startPlaybook = useCallback(
    async (params: {
      name: string;
      playbookIdent: { author: string; package: string; version: string };
      remote?: Array<{
        capability: { author: string; package: string; version: string };
        address: { tcp: string } | { unix: string };
      }>;
      walCapacity?: number;
      successLogRetentionSecs?: number;
      errorLogRetentionSecs?: number;
      socketPath?: string | null;
      inputDir?: string | null;
      outputDir?: string | null;
    }) => {
      try {
        setStartModalOpen(false);
        addLog(
          `Launching playbook "${params.name}" (${params.playbookIdent.author}/${params.playbookIdent.package}@${params.playbookIdent.version})...`,
          "command"
        );
        const msg = (await invoke("start_playbook", {
          name: params.name,
          playbookIdent: params.playbookIdent,
          remote: params.remote || null,
          walCapacity: params.walCapacity ?? null,
          successLogRetentionSecs: params.successLogRetentionSecs ?? null,
          errorLogRetentionSecs: params.errorLogRetentionSecs ?? null,
          playbookSocket: params.socketPath || null,
          inputDir: params.inputDir || null,
          outputDir: params.outputDir || null,
        })) as string;

        addLog(msg, "success");
        await loadPlaybooks();
        await queryDaemonStatus();
      } catch (err) {
        addLog(`Failed to launch playbook: ${err}`, "error");
        showError("Failed to Start Playbook", String(err));
      }
    },
    [addLog, loadPlaybooks, queryDaemonStatus, showError]
  );

  // 7. Call Playbook
  const callPlaybook = useCallback(
    async (name: string, payload: any) => {
      addLog(`Calling playbook "${name}" with payload...`, "command");
      try {
        const res = await invoke("call_playbook", { name, payload });
        addLog(`Playbook "${name}" call succeeded.`, "success");
        return res;
      } catch (err) {
        addLog(`Playbook "${name}" call failed: ${err}`, "error");
        showError(`Call to Playbook "${name}" Failed`, String(err));
        throw err;
      }
    },
    [addLog, showError]
  );

  // Initial and tab-change loads
  useEffect(() => {
    queryDaemonStatus();
    loadCache();
    if (activeTab === "dashboard" || activeTab === "playbooks") {
      loadPlaybooks();
    }
  }, [activeTab, queryDaemonStatus, loadCache, loadPlaybooks]);

  // Periodic polling for daemon status every 10 seconds
  useEffect(() => {
    const interval = setInterval(queryDaemonStatus, 10000);
    return () => clearInterval(interval);
  }, [queryDaemonStatus]);

  // Global refresh
  const handleGlobalRefresh = () => {
    queryDaemonStatus();
    if (activeTab === "dashboard" || activeTab === "playbooks") loadPlaybooks();
    if (activeTab === "repository") loadCache();
  };

  const getPageTitle = () => {
    switch (activeTab) {
      case "dashboard":
        return "Dashboard";
      case "repository":
        return "Repository";
      case "playbooks":
        return "Playbooks";
      case "options":
        return "Options";
      case "logs":
        return "Console Logs";
    }
  };

  return (
    <div className="app-container">
      {/* Sidebar Navigation */}
      <Sidebar
        activeTab={activeTab}
        onTabChange={setActiveTab}
        daemonStatus={daemonStatus}
      />

      {/* Main Content Area */}
      <main className="content-area">
        <header className="top-header">
          <h1>{getPageTitle()}</h1>
          <div className="header-actions">
            <button onClick={handleGlobalRefresh} className="btn btn-secondary">
              Refresh
            </button>
          </div>
        </header>

        {activeTab === "dashboard" && (
          <DashboardTab
            playbooks={playbooks}
            onViewPlaybook={(name) => {
              setSelectedPlaybookName(name);
              setActiveTab("playbooks");
            }}
          />
        )}

        {activeTab === "repository" && (
          <RepositoryTab cacheStatus={cacheStatus} />
        )}

        {activeTab === "playbooks" && (
          <PlaybooksTab
            playbooks={playbooks}
            onStartPlaybookClick={() => setStartModalOpen(true)}
            onStopPlaybookClick={stopPlaybook}
            onSubmitCall={callPlaybook}
            selectedPlaybookName={selectedPlaybookName}
            setSelectedPlaybookName={setSelectedPlaybookName}
          />
        )}

        {activeTab === "logs" && (
          <LogsTab logs={logs} onClearLogs={clearLogs} />
        )}

        {activeTab === "options" && (
          <OptionsTab
            onPurgeAll={purgeCache}
            onPurgeCapabilities={purgeCapabilities}
            onPurgePlaybooks={purgePlaybooks}
            daemonStatus={daemonStatus}
            onQueryStatus={queryDaemonStatus}
            logs={logs}
          />
        )}
      </main>

      {/* Start Playbook Modal */}
      <StartPlaybookModal
        isOpen={startModalOpen}
        availablePlaybooks={cacheStatus?.playbooks ?? []}
        onClose={() => setStartModalOpen(false)}
        onSubmit={startPlaybook}
      />



      {/* Error Notification Toast */}
      {errorNotification && (
        <div className="error-toast">
          <span className="error-toast-icon">⚠️</span>
          <div className="error-toast-content">
            <div className="error-toast-title">{errorNotification.title}</div>
            <div className="error-toast-message">{errorNotification.message}</div>
          </div>
          <button className="error-toast-close" onClick={() => setErrorNotification(null)}>
            &times;
          </button>
        </div>
      )}
    </div>
  );
}
