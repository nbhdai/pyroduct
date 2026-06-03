import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sidebar } from "./components/Sidebar";
import { DashboardTab } from "./components/DashboardTab";
import { RepositoryTab } from "./components/RepositoryTab";
import { PlaybooksTab } from "./components/PlaybooksTab";
import { StartPlaybookModal } from "./components/StartPlaybookModal";
import { CallPlaybookModal } from "./components/CallPlaybookModal";
import { DaemonStatus, CacheStatus, Playbook, LogEntry } from "./types";

export function App() {
  const [activeTab, setActiveTab] = useState<"dashboard" | "repository" | "playbooks">("dashboard");
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
  const [callModalOpen, setCallModalOpen] = useState(false);
  const [callPlaybookName, setCallPlaybookName] = useState("");

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
    }
  }, [addLog]);

  // 2. Load Cache Status
  const loadCache = useCallback(async () => {
    try {
      addLog("Fetching local cache repository status...", "command");
      const res = (await invoke("get_cache_status")) as CacheStatus;
      setCacheStatus(res);
      addLog(
        `Loaded ${res.capabilities.length} capabilities and ${res.modules.length} modules from cache.`,
        "success"
      );
    } catch (err) {
      addLog(`Failed to load cache info: ${err}`, "error");
    }
  }, [addLog]);

  // 3. Load Playbooks
  const loadPlaybooks = useCallback(async () => {
    try {
      addLog("Listing active playbooks from daemon...", "command");
      const res = (await invoke("list_active_playbooks")) as Playbook[];
      setPlaybooks(res);
      addLog(`Retrieved ${res.length} active playbook workers.`, "success");
    } catch (err) {
      addLog(`Failed to list playbooks: ${err}`, "error");
    }
  }, [addLog]);

  // 4. Purge Cache
  const purgeCache = useCallback(async () => {
    const confirmed = window.confirm(
      "Are you sure you want to purge the local cache repository? This deletes cached capability binaries, specifications, and modules."
    );
    if (!confirmed) return;

    try {
      addLog("Purging cache...", "command");
      const msg = (await invoke("purge_cache")) as string;
      addLog(msg, "success");
      await loadCache();
    } catch (err) {
      addLog(`Failed to purge cache: ${err}`, "error");
    }
  }, [addLog, loadCache]);

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
      }
    },
    [addLog, loadPlaybooks, queryDaemonStatus]
  );

  // 6. Start Playbook
  const startPlaybook = useCallback(
    async (params: {
      name: string;
      configPath: string;
      socketPath: string | null;
      inputDir: string | null;
      outputDir: string | null;
    }) => {
      try {
        addLog(`Launching playbook "${params.name}" from config "${params.configPath}"...`, "command");
        const msg = (await invoke("start_playbook", {
          name: params.name,
          configPath: params.configPath,
          playbookSocket: params.socketPath,
          inputDir: params.inputDir,
          outputDir: params.outputDir,
        })) as string;

        addLog(msg, "success");
        setStartModalOpen(false);
        await loadPlaybooks();
        await queryDaemonStatus();
      } catch (err) {
        addLog(`Failed to launch playbook: ${err}`, "error");
        alert(`Failed to launch playbook: ${err}`);
      }
    },
    [addLog, loadPlaybooks, queryDaemonStatus]
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
        throw err;
      }
    },
    [addLog]
  );

  // Initial and tab-change loads
  useEffect(() => {
    queryDaemonStatus();
    if (activeTab === "repository") {
      loadCache();
    } else if (activeTab === "playbooks") {
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
    if (activeTab === "repository") loadCache();
    if (activeTab === "playbooks") loadPlaybooks();
  };

  const getPageTitle = () => {
    switch (activeTab) {
      case "dashboard":
        return "Dashboard";
      case "repository":
        return "Repository";
      case "playbooks":
        return "Playbooks";
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
              🔄 Refresh
            </button>
          </div>
        </header>

        {activeTab === "dashboard" && (
          <DashboardTab
            daemonStatus={daemonStatus}
            onQueryStatus={queryDaemonStatus}
            onPurgeCache={purgeCache}
            logs={logs}
          />
        )}

        {activeTab === "repository" && (
          <RepositoryTab cacheStatus={cacheStatus} onPurgeCache={purgeCache} />
        )}

        {activeTab === "playbooks" && (
          <PlaybooksTab
            playbooks={playbooks}
            onStartPlaybookClick={() => setStartModalOpen(true)}
            onCallPlaybookClick={(name) => {
              setCallPlaybookName(name);
              setCallModalOpen(true);
            }}
            onStopPlaybookClick={stopPlaybook}
          />
        )}
      </main>

      {/* Start Playbook Modal */}
      <StartPlaybookModal
        isOpen={startModalOpen}
        onClose={() => setStartModalOpen(false)}
        onSubmit={startPlaybook}
      />

      {/* Call Playbook Modal */}
      <CallPlaybookModal
        isOpen={callModalOpen}
        playbookName={callPlaybookName}
        onClose={() => setCallModalOpen(false)}
        onSubmit={callPlaybook}
      />
    </div>
  );
}
