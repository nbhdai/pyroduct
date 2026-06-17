import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PyroductConfig, DaemonStatus, LogEntry, GuiSettings, DaemonConnection } from "../types";

interface OptionsTabProps {
  onPurgeAll: () => Promise<void>;
  onPurgeCapabilities: () => Promise<void>;
  onPurgePlaybooks: () => Promise<void>;
  daemonStatus: DaemonStatus;
  onQueryStatus: () => void;
  logs: LogEntry[];
}

export function OptionsTab({
  onPurgeAll,
  onPurgeCapabilities,
  onPurgePlaybooks,
  daemonStatus,
  onQueryStatus,
  logs,
}: OptionsTabProps) {
  const [config, setConfig] = useState<PyroductConfig>({ author: "", build_slots: 4 });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
  const [subTab, setSubTab] = useState<"config" | "daemon">("config");
  const consoleEndRef = useRef<HTMLDivElement>(null);

  // Daemon settings state
  const [guiSettings, setGuiSettings] = useState<GuiSettings>({ daemons: {} });
  const [newDaemonName, setNewDaemonName] = useState("");
  const [newDaemonType, setNewDaemonType] = useState<"unix" | "tcp">("tcp");
  const [newDaemonTarget, setNewDaemonTarget] = useState("");
  const [settingsLoading, setSettingsLoading] = useState(true);

  // Load Pyroduct config
  useEffect(() => {
    let active = true;
    invoke("get_pyroduct_config")
      .then((res) => {
        if (active) {
          const cfg = res as any;
          setConfig({
            author: cfg.author || "",
            build_slots: cfg.build_slots !== undefined ? cfg.build_slots : 4,
          });
          setLoading(false);
        }
      })
      .catch((err) => {
        if (active) {
          setMessage({ text: `Failed to load configuration: ${err}`, type: "error" });
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, []);

  // Load GUI settings (daemon list/selection)
  const loadGuiSettings = async () => {
    try {
      setSettingsLoading(true);
      const settings = (await invoke("get_gui_settings")) as GuiSettings;
      setGuiSettings(settings);
    } catch (err) {
      console.error("Failed to load GUI settings", err);
    } finally {
      setSettingsLoading(false);
    }
  };

  useEffect(() => {
    loadGuiSettings();
  }, []);

  useEffect(() => {
    if (subTab === "daemon" && consoleEndRef.current) {
      consoleEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, subTab]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setMessage(null);

    try {
      await invoke("update_pyroduct_config", {
        author: config.author,
        buildSlots: config.build_slots ? Number(config.build_slots) : null,
      });
      setMessage({ text: "Configuration saved successfully.", type: "success" });
    } catch (err) {
      setMessage({ text: `Failed to save configuration: ${err}`, type: "error" });
    } finally {
      setSaving(false);
    }
  };

  const handleSelectDaemon = async (name: string | undefined) => {
    const updated = { ...guiSettings, selected_daemon: name };
    try {
      await invoke("update_gui_settings", { settings: updated });
      setGuiSettings(updated);
      onQueryStatus();
    } catch (err) {
      alert(`Failed to select daemon: ${err}`);
    }
  };

  const handleAddDaemon = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newDaemonName.trim() || !newDaemonTarget.trim()) return;

    const name = newDaemonName.trim();
    const target = newDaemonTarget.trim();

    const conn: DaemonConnection = newDaemonType === "unix"
      ? { type: "unix", path: target }
      : { type: "tcp", address: target };

    const updatedDaemons = { ...guiSettings.daemons, [name]: conn };
    const updated = { ...guiSettings, daemons: updatedDaemons };

    try {
      await invoke("update_gui_settings", { settings: updated });
      setGuiSettings(updated);
      setNewDaemonName("");
      setNewDaemonTarget("");
    } catch (err) {
      alert(`Failed to add daemon: ${err}`);
    }
  };

  const handleRemoveDaemon = async (name: string) => {
    const confirmed = window.confirm(`Are you sure you want to remove connection "${name}"?`);
    if (!confirmed) return;

    const updatedDaemons = { ...guiSettings.daemons };
    delete updatedDaemons[name];

    let nextSelected = guiSettings.selected_daemon;
    if (nextSelected === name) {
      nextSelected = undefined;
    }

    const updated = { ...guiSettings, daemons: updatedDaemons, selected_daemon: nextSelected };
    try {
      await invoke("update_gui_settings", { settings: updated });
      setGuiSettings(updated);
      onQueryStatus();
    } catch (err) {
      alert(`Failed to remove daemon connection: ${err}`);
    }
  };

  const handlePurgeClick = async (action: () => Promise<void>, confirmMsg: string) => {
    const confirmed = window.confirm(confirmMsg);
    if (!confirmed) return;
    try {
      await action();
    } catch (err) {
      alert(`Purge action failed: ${err}`);
    }
  };

  const getStatusBadgeClass = () => {
    if (daemonStatus.status === "online") return "value badge badge-online";
    if (daemonStatus.status === "offline") return "value badge badge-offline";
    return "value badge badge-offline";
  };

  const getStatusText = () => {
    if (daemonStatus.status === "online") return "Online";
    if (daemonStatus.status === "offline") return "Offline";
    return "Error";
  };

  const resolvedDefaultPath = daemonStatus.socket_path || "~/.pyroduct/control";

  return (
    <div className="tab-content active">
      {/* Sub Tabs */}
      <div className="tabs-sub" style={{ marginBottom: "25px" }}>
        <button
          className={`sub-tab-btn ${subTab === "config" ? "active" : ""}`}
          onClick={() => setSubTab("config")}
        >
          Configuration & Cache
        </button>
        <button
          className={`sub-tab-btn ${subTab === "daemon" ? "active" : ""}`}
          onClick={() => setSubTab("daemon")}
        >
          Daemon Connections
        </button>
      </div>

      {subTab === "config" && (
        <div className="options-layout">
          {/* Configuration Card */}
          <div className="card">
            <h2>Pyroduct Configuration</h2>
            {loading ? (
              <div className="flex items-center justify-center p-20">
                <div className="spinner"></div>
              </div>
            ) : (
              <form onSubmit={handleSave} className="mt-15">
                <div className="form-group">
                  <label htmlFor="cfg-author">Author Identity</label>
                  <input
                    id="cfg-author"
                    type="text"
                    value={config.author}
                    onChange={(e) => setConfig({ ...config, author: e.target.value })}
                    placeholder="e.g. anon"
                    required
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="cfg-slots">Build Slots (Concurrency)</label>
                  <input
                    id="cfg-slots"
                    type="number"
                    min={1}
                    max={32}
                    value={config.build_slots ?? ""}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        build_slots: e.target.value ? Number(e.target.value) : undefined,
                      })
                    }
                    placeholder="e.g. 4"
                  />
                </div>

                {message && (
                  <div
                    className={`card p-12 mb-15 text-sm ${
                      message.type === "success"
                        ? "border-success bg-success-glow text-success"
                        : "border-danger bg-danger-glow text-danger"
                    }`}
                    style={{ borderRadius: "8px", border: "1px solid" }}
                  >
                    {message.text}
                  </div>
                )}

                <button type="submit" className="btn btn-primary" disabled={saving}>
                  {saving ? "Saving..." : "Save Configuration"}
                </button>
              </form>
            )}
          </div>

          {/* Maintenance / Purge Actions Card */}
          <div className="card">
            <h2>Cache Maintenance</h2>
            <p className="text-muted text-sm mt-5 mb-20">
              Manage your local cache storage. Purging will force the GUI and daemon to download or recompile dependencies when next requested.
            </p>

            <div className="info-list">
              {/* Purge Capabilities */}
              <div className="info-row flex justify-between items-center py-12" style={{ borderBottom: "1px solid var(--bg-card-border)", paddingBottom: "16px" }}>
                <div>
                  <div className="font-semibold text-main">Capabilities Cache</div>
                  <div className="text-muted text-xs mt-2">Purges cached capabilities and interface JSONs.</div>
                </div>
                <button
                  onClick={() =>
                    handlePurgeClick(
                      onPurgeCapabilities,
                      "Are you sure you want to purge the capabilities cache? This deletes all cached capability binaries and specifications."
                    )
                  }
                  className="btn btn-danger btn-sm"
                >
                  Purge Capabilities
                </button>
              </div>

              {/* Purge Playbooks */}
              <div className="info-row flex justify-between items-center py-12" style={{ borderBottom: "1px solid var(--bg-card-border)", paddingTop: "16px", paddingBottom: "16px" }}>
                <div>
                  <div className="font-semibold text-main">Playbooks Cache</div>
                  <div className="text-muted text-xs mt-2">Purges compiled playbooks cache.</div>
                </div>
                <button
                  onClick={() =>
                    handlePurgeClick(
                      onPurgePlaybooks,
                      "Are you sure you want to purge playbooks cache? This deletes all compiled playbooks."
                    )
                  }
                  className="btn btn-danger btn-sm"
                >
                  Purge Playbooks
                </button>
              </div>

              {/* Purge All */}
              <div className="info-row flex justify-between items-center py-12" style={{ paddingTop: "16px" }}>
                <div>
                  <div className="font-semibold text-main text-danger">Entire Cache</div>
                  <div className="text-muted text-xs mt-2">Deletes all cached items including capabilities, playbooks, and interfaces.</div>
                </div>
                <button
                  onClick={() =>
                    handlePurgeClick(
                      onPurgeAll,
                      "Are you sure you want to purge the entire cache? This will delete all locally cached capabilities, playbooks, and specifications."
                    )
                  }
                  className="btn btn-danger btn-sm"
                  style={{ background: "var(--color-danger)", color: "white" }}
                >
                  Purge Entire Cache
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {subTab === "daemon" && (
        <div className="grid-layout animate-fade-in" style={{ gridTemplateColumns: "1.3fr 1fr", gap: "25px" }}>
          
          {/* Left Column: Connection Selector & Management */}
          <div className="flex-col-gap" style={{ display: "flex", flexDirection: "column", gap: "25px" }}>
            
            {/* Connection List */}
            <div className="card">
              <h2>Daemon Connection Selector</h2>
              <p className="text-muted text-sm mt-5 mb-20">
                Select the daemon instance the GUI commands will communicate with.
              </p>

              {settingsLoading ? (
                <div style={{ display: "flex", justifyContent: "center", padding: "30px" }}>
                  <div className="spinner"></div>
                </div>
              ) : (
                <div className="info-list" style={{ gap: "14px" }}>
                  {/* Default Local Daemon */}
                  <div 
                    className="connection-item"
                    style={{
                      padding: "16px",
                      borderRadius: "10px",
                      backgroundColor: !guiSettings.selected_daemon ? "rgba(255, 92, 0, 0.05)" : "rgba(255, 255, 255, 0.02)",
                      border: !guiSettings.selected_daemon ? "1px solid var(--color-primary)" : "1px solid var(--bg-card-border)",
                      boxShadow: !guiSettings.selected_daemon ? "0 0 10px rgba(255, 92, 0, 0.1)" : "none",
                      display: "flex",
                      justifyContent: "between",
                      alignItems: "center",
                      transition: "all 0.2s ease"
                    }}
                  >
                    <div style={{ display: "flex", flexDirection: "column", gap: "4px", flexGrow: 1 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                        <span style={{ fontWeight: "700", color: "var(--text-main)" }}>Default (Local Unix Socket)</span>
                        <span className="badge badge-system" style={{ fontSize: "10px", padding: "2px 6px" }}>UNIX</span>
                        {!guiSettings.selected_daemon && (
                          <span className="badge badge-online" style={{ fontSize: "10px", padding: "2px 6px" }}>ACTIVE</span>
                        )}
                      </div>
                      <span className="code-text" style={{ fontSize: "11px", alignSelf: "flex-start", opacity: 0.8 }}>
                        {resolvedDefaultPath}
                      </span>
                    </div>

                    {guiSettings.selected_daemon && (
                      <button 
                        onClick={() => handleSelectDaemon(undefined)}
                        className="btn btn-secondary btn-sm"
                        style={{ padding: "6px 12px", fontSize: "12px", marginLeft: "12px" }}
                      >
                        Activate
                      </button>
                    )}
                  </div>

                  {/* Configured Daemons */}
                  {Object.entries(guiSettings.daemons || {}).map(([name, conn]) => {
                    const isActive = guiSettings.selected_daemon === name;
                    const pathOrAddress = conn.type === "unix" ? (conn.path || "") : (conn.address || "");
                    
                    return (
                      <div 
                        key={name}
                        className="connection-item"
                        style={{
                          padding: "16px",
                          borderRadius: "10px",
                          backgroundColor: isActive ? "rgba(255, 92, 0, 0.05)" : "rgba(255, 255, 255, 0.02)",
                          border: isActive ? "1px solid var(--color-primary)" : "1px solid var(--bg-card-border)",
                          boxShadow: isActive ? "0 0 10px rgba(255, 92, 0, 0.1)" : "none",
                          display: "flex",
                          justifyContent: "between",
                          alignItems: "center",
                          transition: "all 0.2s ease"
                        }}
                      >
                        <div style={{ display: "flex", flexDirection: "column", gap: "4px", flexGrow: 1 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                            <span style={{ fontWeight: "700", color: "var(--text-main)" }}>{name}</span>
                            <span className="badge badge-system" style={{ fontSize: "10px", padding: "2px 6px", textTransform: "uppercase" }}>
                              {conn.type}
                            </span>
                            {isActive && (
                              <span className="badge badge-online" style={{ fontSize: "10px", padding: "2px 6px" }}>ACTIVE</span>
                            )}
                          </div>
                          <span className="code-text" style={{ fontSize: "11px", alignSelf: "flex-start", opacity: 0.8 }}>
                            {pathOrAddress}
                          </span>
                        </div>

                        <div style={{ display: "flex", gap: "8px", marginLeft: "12px" }}>
                          {!isActive && (
                            <button 
                              onClick={() => handleSelectDaemon(name)}
                              className="btn btn-secondary btn-sm"
                              style={{ padding: "6px 12px", fontSize: "12px" }}
                            >
                              Activate
                            </button>
                          )}
                          <button 
                            onClick={() => handleRemoveDaemon(name)}
                            className="btn btn-danger btn-sm"
                            style={{ padding: "6px 12px", fontSize: "12px", minWidth: "auto" }}
                          >
                            Delete
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Add Connection */}
            <div className="card">
              <h2>Add Remote Connection</h2>
              <form onSubmit={handleAddDaemon} style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "15px" }}>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label htmlFor="daemon-name">Connection Friendly Name</label>
                    <input
                      id="daemon-name"
                      type="text"
                      value={newDaemonName}
                      onChange={(e) => setNewDaemonName(e.target.value)}
                      placeholder="e.g. Staging Daemon"
                      required
                    />
                  </div>

                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label htmlFor="daemon-type">Type</label>
                    <select
                      id="daemon-type"
                      value={newDaemonType}
                      onChange={(e) => setNewDaemonType(e.target.value as "unix" | "tcp")}
                      style={{
                        width: "100%",
                        padding: "12px 14px",
                        backgroundColor: "rgba(255, 255, 255, 0.03)",
                        border: "1px solid var(--bg-card-border)",
                        borderRadius: "8px",
                        color: "var(--text-main)",
                        fontFamily: "inherit",
                        fontSize: "14px",
                        outline: "none"
                      }}
                    >
                      <option value="tcp" style={{ backgroundColor: "#131316", color: "white" }}>TCP Address</option>
                      <option value="unix" style={{ backgroundColor: "#131316", color: "white" }}>UNIX Domain Socket</option>
                    </select>
                  </div>
                </div>

                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label htmlFor="daemon-target">
                    {newDaemonType === "unix" ? "Control Socket Path" : "TCP Address & Port"}
                  </label>
                  <input
                    id="daemon-target"
                    type="text"
                    value={newDaemonTarget}
                    onChange={(e) => setNewDaemonTarget(e.target.value)}
                    placeholder={newDaemonType === "unix" ? "/var/run/pyro-daemon/control" : "127.0.0.1:9099"}
                    required
                  />
                </div>

                <button type="submit" className="btn btn-primary" style={{ alignSelf: "flex-end", marginTop: "5px" }}>
                  Add Connection
                </button>
              </form>
            </div>

          </div>

          {/* Right Column: Status & Quick Actions */}
          <div className="flex-col-gap" style={{ display: "flex", flexDirection: "column", gap: "25px" }}>
            
            {/* Status Card */}
            <div className="card status-card">
              <h2>Daemon Information</h2>
              <div className="info-list">
                <div className="info-row">
                  <span className="label">Active Connection</span>
                  <span className="value font-semibold text-main" style={{ color: "var(--color-primary)" }}>
                    {daemonStatus.daemon_name || "Default (Local)"}
                  </span>
                </div>
                <div className="info-row">
                  <span className="label">Daemon Status</span>
                  <span className={getStatusBadgeClass()}>{getStatusText()}</span>
                </div>
                <div className="info-row">
                  <span className="label">Active Workers</span>
                  <span className="value">{daemonStatus.active_workers ?? "0"}</span>
                </div>
                <div className="info-row">
                  <span className="label">Daemon Version</span>
                  <span className="value">{daemonStatus.version ?? "-"}</span>
                </div>
                <div className="info-row" style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: "8px", borderBottom: "none" }}>
                  <span className="label">Control Target</span>
                  <span className="value code-text" style={{ wordBreak: "break-all", width: "100%", textAlign: "left" }}>
                    {daemonStatus.socket_path ?? "-"}
                  </span>
                </div>
              </div>
            </div>

            {/* Actions Card */}
            <div className="card actions-card">
              <h2>Quick Actions</h2>
              <div className="action-buttons">
                <button onClick={onQueryStatus} className="btn btn-primary btn-block">
                  Query Status
                </button>
              </div>
            </div>

          </div>

          {/* Event Console */}
          <div className="card log-card mt-20" style={{ gridColumn: "1 / -1" }}>
            <h2>Event Console</h2>
            <div className="console-box" style={{ maxHeight: "300px", overflowY: "auto" }}>
              {logs.map((log, index) => (
                <div key={index} className={`log-line ${log.type}`}>
                  [{log.time}] [{log.type}] {log.message}
                </div>
              ))}
              <div ref={consoleEndRef} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
