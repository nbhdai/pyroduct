import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PyroductConfig } from "../types";

interface OptionsTabProps {
  onPurgeAll: () => Promise<void>;
  onPurgeCapabilities: () => Promise<void>;
  onPurgeModules: () => Promise<void>;
}

export function OptionsTab({ onPurgeAll, onPurgeCapabilities, onPurgeModules }: OptionsTabProps) {
  const [config, setConfig] = useState<PyroductConfig>({ author: "", build_slots: 4 });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);

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

  return (
    <div className="tab-content active">
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

            {/* Purge Modules */}
            <div className="info-row flex justify-between items-center py-12" style={{ borderBottom: "1px solid var(--bg-card-border)", paddingTop: "16px", paddingBottom: "16px" }}>
              <div>
                <div className="font-semibold text-main">Modules Cache</div>
                <div className="text-muted text-xs mt-2">Purges modules and compiled playbooks.</div>
              </div>
              <button
                onClick={() =>
                  handlePurgeClick(
                    onPurgeModules,
                    "Are you sure you want to purge modules cache? This deletes all compiled playbooks."
                  )
                }
                className="btn btn-danger btn-sm"
              >
                Purge Modules
              </button>
            </div>

            {/* Purge All */}
            <div className="info-row flex justify-between items-center py-12" style={{ paddingTop: "16px" }}>
              <div>
                <div className="font-semibold text-main text-danger">Entire Cache</div>
                <div className="text-muted text-xs mt-2">Deletes all cached items including capabilities, modules, and interfaces.</div>
              </div>
              <button
                onClick={() =>
                  handlePurgeClick(
                    onPurgeAll,
                    "Are you sure you want to purge the entire cache? This will delete all locally cached capabilities, modules, and specifications."
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
    </div>
  );
}
