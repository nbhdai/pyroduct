import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Playbook, CallbackMapping } from "../types";

interface CallbacksTabProps {
  playbookName: string;
  playbooks: Playbook[];
}

export function CallbacksTab({ playbookName, playbooks }: CallbacksTabProps) {
  const [callbacks, setCallbacks] = useState<CallbackMapping[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Form states
  const [callbackType, setCallbackType] = useState<"playbook" | "http" | "socket">("playbook");
  const [targetPlaybook, setTargetPlaybook] = useState("");
  const [targetHttp, setTargetHttp] = useState("");
  const [targetSocket, setTargetSocket] = useState("");

  const fetchCallbacks = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = (await invoke("list_callbacks", { source: playbookName })) as CallbackMapping[];
      setCallbacks(res || []);
    } catch (err) {
      setError(String(err));
      console.error("Failed to fetch callbacks:", err);
    } finally {
      setLoading(false);
    }
  }, [playbookName]);

  useEffect(() => {
    fetchCallbacks();
  }, [fetchCallbacks]);

  // Set default target playbook when playbooks load or change
  const otherPlaybooks = playbooks.filter((p) => p.name !== playbookName);
  useEffect(() => {
    if (otherPlaybooks.length > 0 && !targetPlaybook) {
      setTargetPlaybook(otherPlaybooks[0].name);
    }
  }, [otherPlaybooks, targetPlaybook]);

  const handleAddCallback = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setError(null);

    try {
      if (callbackType === "playbook") {
        if (!targetPlaybook) {
          throw new Error("Please select a target playbook.");
        }
        await invoke("add_playbook_callback", {
          source: playbookName,
          targetPlaybook,
        });
      } else if (callbackType === "http") {
        if (!targetHttp.trim()) {
          throw new Error("Please enter an HTTP URL.");
        }
        await invoke("add_http_callback", {
          source: playbookName,
          url: targetHttp.trim(),
        });
        setTargetHttp("");
      } else if (callbackType === "socket") {
        if (!targetSocket.trim()) {
          throw new Error("Please enter a socket address/path.");
        }
        await invoke("add_socket_callback", {
          source: playbookName,
          socketPath: targetSocket.trim(),
        });
        setTargetSocket("");
      }

      await fetchCallbacks();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handleDeleteCallback = async (uuid: string) => {
    if (!confirm("Are you sure you want to delete this callback?")) return;
    setError(null);
    try {
      await invoke("delete_callback", { uuidStr: uuid });
      await fetchCallbacks();
    } catch (err) {
      setError(`Failed to delete callback: ${err}`);
    }
  };

  const getBadgeStyle = (type: string) => {
    switch (type) {
      case "playbook":
        return {
          backgroundColor: "rgba(37, 99, 235, 0.15)",
          color: "#60a5fa",
          border: "1px solid rgba(37, 99, 235, 0.3)",
        };
      case "http":
        return {
          backgroundColor: "rgba(16, 185, 129, 0.15)",
          color: "#34d399",
          border: "1px solid rgba(16, 185, 129, 0.3)",
        };
      case "socket":
        return {
          backgroundColor: "rgba(245, 158, 11, 0.15)",
          color: "#fbbf24",
          border: "1px solid rgba(245, 158, 11, 0.3)",
        };
      default:
        return {};
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
      {error && (
        <div className="card border-danger bg-danger-glow p-15" style={{ borderRadius: "8px" }}>
          <p className="text-danger" style={{ margin: 0, fontSize: "14px" }}>
            ⚠️ {error}
          </p>
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "24px" }} className="responsive-playbook-grid">
        {/* Callbacks List Card */}
        <div className="card" style={{ padding: "24px", display: "flex", flexDirection: "column", gap: "15px" }}>
          <h3 style={{ fontSize: "16px", fontWeight: 600, margin: 0 }}>Registered Callbacks</h3>
          
          {loading ? (
            <div style={{ display: "flex", justifyContent: "center", padding: "40px 0" }}>
              <div className="spinner"></div>
            </div>
          ) : callbacks.length === 0 ? (
            <div style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              padding: "40px 20px",
              color: "var(--text-muted)",
              textAlign: "center",
              backgroundColor: "rgba(255, 255, 255, 0.01)",
              borderRadius: "8px",
              border: "1px dashed var(--bg-card-border)",
              flexGrow: 1
            }}>
              <span style={{ fontSize: "28px", marginBottom: "8px", opacity: 0.3 }}>🔗</span>
              <p style={{ fontSize: "13px", fontWeight: 500, margin: 0 }}>No callbacks registered.</p>
              <p style={{ fontSize: "11px", opacity: 0.7, marginTop: "4px" }}>
                Outputs generated by this worker will not be forwarded automatically.
              </p>
            </div>
          ) : (
            <div className="table-container" style={{ overflowX: "auto", flexGrow: 1 }}>
              <table className="table" style={{ width: "100%" }}>
                <thead>
                  <tr>
                    <th style={{ width: "100px" }}>Type</th>
                    <th>Target Destination</th>
                    <th style={{ width: "60px", textAlign: "right" }}>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {callbacks.map((cb) => (
                    <tr key={cb.uuid}>
                      <td>
                        <span
                          className="badge"
                          style={{
                            padding: "4px 8px",
                            fontSize: "11px",
                            borderRadius: "4px",
                            textTransform: "uppercase",
                            fontWeight: 600,
                            letterSpacing: "0.03em",
                            display: "inline-block",
                            ...getBadgeStyle(cb.callback_type),
                          }}
                        >
                          {cb.callback_type}
                        </span>
                      </td>
                      <td style={{ verticalAlign: "middle" }}>
                        <span className="code-text" style={{ fontSize: "12px", wordBreak: "break-all" }}>
                          {cb.target}
                        </span>
                      </td>
                      <td style={{ textAlign: "right" }}>
                        <button
                          onClick={() => handleDeleteCallback(cb.uuid)}
                          className="btn btn-danger btn-sm"
                          style={{ padding: "4px 8px", fontSize: "11px" }}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Add Callback Card */}
        <div className="card" style={{ padding: "24px" }}>
          <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "20px" }}>Add Output Callback</h3>
          
          <form onSubmit={handleAddCallback} style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
            <div className="form-group">
              <label>Callback Type</label>
              <div style={{ display: "flex", gap: "8px", marginTop: "4px" }}>
                {(["playbook", "http", "socket"] as const).map((type) => (
                  <button
                    key={type}
                    type="button"
                    onClick={() => setCallbackType(type)}
                    className={`btn ${callbackType === type ? "btn-primary" : "btn-secondary"}`}
                    style={{
                      flex: 1,
                      padding: "8px 12px",
                      fontSize: "13px",
                      fontWeight: 600,
                      textTransform: "capitalize",
                    }}
                  >
                    {type}
                  </button>
                ))}
              </div>
            </div>

            {callbackType === "playbook" && (
              <div className="form-group">
                <label htmlFor="target-playbook">Target Playbook Worker</label>
                {otherPlaybooks.length === 0 ? (
                  <p className="text-muted" style={{ fontSize: "12px", margin: "8px 0 0 0" }}>
                    No other active playbooks are running to chain to. Start another worker first.
                  </p>
                ) : (
                  <select
                    id="target-playbook"
                    value={targetPlaybook}
                    onChange={(e) => setTargetPlaybook(e.target.value)}
                    style={{ marginTop: "4px" }}
                    required
                  >
                    {otherPlaybooks.map((pb) => (
                      <option key={pb.name} value={pb.name}>
                        {pb.name}
                      </option>
                    ))}
                  </select>
                )}
              </div>
            )}

            {callbackType === "http" && (
              <div className="form-group">
                <label htmlFor="target-http">Webhook URL</label>
                <input
                  type="url"
                  id="target-http"
                  value={targetHttp}
                  onChange={(e) => setTargetHttp(e.target.value)}
                  placeholder="e.g. http://127.0.0.1:8080/webhook"
                  style={{ marginTop: "4px" }}
                  required
                />
              </div>
            )}

            {callbackType === "socket" && (
              <div className="form-group">
                <label htmlFor="target-socket">Socket Path / TCP Address</label>
                <input
                  type="text"
                  id="target-socket"
                  value={targetSocket}
                  onChange={(e) => setTargetSocket(e.target.value)}
                  placeholder="e.g. 127.0.0.1:12345 or /tmp/receiver.sock"
                  style={{ marginTop: "4px" }}
                  required
                />
              </div>
            )}

            <button
              type="submit"
              disabled={submitting || (callbackType === "playbook" && otherPlaybooks.length === 0)}
              className="btn btn-success"
              style={{
                width: "100%",
                padding: "10px 16px",
                fontSize: "14px",
                fontWeight: 600,
                marginTop: "10px",
              }}
            >
              {submitting ? "Registering..." : "Add Callback"}
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}
