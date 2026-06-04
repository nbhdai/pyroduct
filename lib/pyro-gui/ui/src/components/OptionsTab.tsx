import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PyroductConfig, DaemonStatus, LogEntry } from "../types";

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
          Daemon Status
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
        <div className="grid-layout animate-fade-in">
          {/* Status Card */}
          <div className="card status-card">
            <h2>Daemon Information</h2>
            <div className="info-list">
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
              <div className="info-row">
                <span className="label">Control Socket</span>
                <span className="value code-text">{daemonStatus.socket_path ?? "-"}</span>
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
