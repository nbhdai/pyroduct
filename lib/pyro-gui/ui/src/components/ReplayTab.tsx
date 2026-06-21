import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Playbook } from "../types";

interface ReplayStatus {
  running: boolean;
  total_rows: number;
  rows_completed: number;
  successes: number;
  errors: number;
  current_file: string;
}

interface ReplayTabProps {
  playbooks?: Playbook[];
  playbookName?: string;
  onSuccess?: () => void;
}

export function ReplayTab({ playbooks, playbookName, onSuccess }: ReplayTabProps) {
  const [folderPath, setFolderPath] = useState("");
  const [mode, setMode] = useState<"timed" | "parallel">("timed");
  const [intervalMs, setIntervalMs] = useState(100);
  const [wiggleMs, setWiggleMs] = useState(20);
  const [concurrency, setConcurrency] = useState(4);
  const [starting, setStarting] = useState(false);
  const [status, setStatus] = useState<ReplayStatus | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const eligiblePlaybooks = (playbooks || []).filter(
    (p) => p.spec?.func?.kind !== "session" && p.spec?.func?.kind !== "session_diff"
  );

  const [selectedPlaybookName, setSelectedPlaybookName] = useState(playbookName || "");

  // Sync selectedPlaybookName if prop changes
  useEffect(() => {
    if (playbookName) {
      setSelectedPlaybookName(playbookName);
    }
  }, [playbookName]);

  // Auto-select first eligible playbook if none selected and not passed via prop
  useEffect(() => {
    if (!selectedPlaybookName && eligiblePlaybooks.length > 0 && !playbookName) {
      setSelectedPlaybookName(eligiblePlaybooks[0].name);
    }
  }, [eligiblePlaybooks, selectedPlaybookName, playbookName]);

  // Poll replay status while running
  useEffect(() => {
    if (!selectedPlaybookName) {
      setStatus(null);
      return;
    }
    const poll = async () => {
      try {
        const res = (await invoke("get_replay_status", {
          playbookName: selectedPlaybookName,
        })) as ReplayStatus;
        setStatus(res);
        if (!res.running && res.rows_completed > 0 && onSuccess) {
          onSuccess();
        }
      } catch {
        // Ignore poll errors silently
      }
    };

    // Initial fetch
    poll();

    pollRef.current = setInterval(poll, 1000);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [selectedPlaybookName, onSuccess]);

  const handleStart = async () => {
    if (!selectedPlaybookName) {
      setErrorMsg("Please select a target playbook worker.");
      return;
    }
    if (!folderPath.trim()) {
      setErrorMsg("Please enter a folder path.");
      return;
    }
    setStarting(true);
    setErrorMsg(null);
    try {
      let res = mode === "parallel"
        ? ((await invoke("start_parallel_folder_replay", {
            playbookName: selectedPlaybookName,
            folderPath: folderPath.trim(),
            concurrency,
          })) as { total_rows: number })
        : ((await invoke("start_folder_replay", {
            playbookName: selectedPlaybookName,
            folderPath: folderPath.trim(),
            intervalMs,
            wiggleMs,
          })) as { total_rows: number });
      setStatus({
        running: true,
        total_rows: res.total_rows,
        rows_completed: 0,
        successes: 0,
        errors: 0,
        current_file: "",
      });
    } catch (err: any) {
      setErrorMsg(`Failed to start replay: ${err}`);
    } finally {
      setStarting(false);
    }
  };

  const handleStop = async () => {
    if (!selectedPlaybookName) return;
    try {
      await invoke("stop_folder_replay", { playbookName: selectedPlaybookName });
    } catch (err: any) {
      setErrorMsg(`Failed to stop replay: ${err}`);
    }
  };

  const isRunning = status?.running === true;
  const isCompleted = status && !status.running && status.rows_completed > 0;
  const progressPct =
    status && status.total_rows > 0
      ? Math.round((status.rows_completed / status.total_rows) * 100)
      : 0;

  // Estimated time remaining
  const getEta = () => {
    if (!status || !isRunning || status.rows_completed === 0) return null;
    const remaining = status.total_rows - status.rows_completed;
    const avgMs = intervalMs; // approximate
    const etaSec = Math.round((remaining * avgMs) / 1000);
    if (etaSec < 60) return `~${etaSec}s remaining`;
    const mins = Math.floor(etaSec / 60);
    const secs = etaSec % 60;
    return `~${mins}m ${secs}s remaining`;
  };

  if (!playbookName && eligiblePlaybooks.length === 0) {
    return (
      <div className="card" style={{ padding: "24px", textAlign: "center" }}>
        <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "12px" }}>
          Folder Replay
        </h3>
        <p className="text-muted text-sm" style={{ marginBottom: "20px", lineHeight: "1.5" }}>
          Replay data files from a folder to an active playbook worker at a fixed rate.
        </p>
        <div
          style={{
            padding: "24px",
            borderRadius: "8px",
            border: "1px dashed var(--bg-card-border)",
            backgroundColor: "rgba(255,255,255,0.01)",
            display: "inline-block",
            margin: "0 auto 10px auto",
            maxWidth: "400px",
          }}
        >
          <div style={{ fontSize: "24px", marginBottom: "10px" }}>⚠️</div>
          <div style={{ fontWeight: 600, fontSize: "14px", marginBottom: "6px" }}>
            No Eligible Workers Running
          </div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: "1.4" }}>
            Replay requires a running non-session playbook worker. Start a worker in the <strong>Playbooks</strong> tab first.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="card" style={{ marginBottom: "20px" }}>
      <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "6px" }}>
        Folder Replay
      </h3>
      <p
        className="text-muted text-sm mt-5 mb-20"
        style={{ lineHeight: "1.5" }}
      >
        Replay data files from a folder to this playbook. Choose <strong>Timed</strong> mode
        for rate-controlled replay with optional jitter, or <strong>Parallel</strong> mode
        to process as fast as possible with K concurrent jobs. Supports CSV, JSON,
        JSONL, Parquet, and Arrow IPC files. Files are processed in alphabetical order.
      </p>

      {errorMsg && (
        <div
          className="card p-12 mb-15 text-sm"
          style={{
            borderRadius: "8px",
            border: "1px solid var(--color-danger)",
            backgroundColor: "var(--color-danger-glow)",
            color: "var(--color-danger)",
          }}
        >
          {errorMsg}
        </div>
      )}

      {/* Configuration Section */}
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "16px",
          marginBottom: "20px",
        }}
      >
        {/* Playbook Selection */}
        {!playbookName && (
          <div>
            <label
              htmlFor="replay-playbook-select"
              style={{
                display: "block",
                marginBottom: "6px",
                fontWeight: 600,
                fontSize: "13px",
              }}
            >
              Target Playbook Worker
            </label>
            <select
              id="replay-playbook-select"
              value={selectedPlaybookName}
              onChange={(e) => setSelectedPlaybookName(e.target.value)}
              disabled={isRunning || starting}
              style={{
                width: "100%",
                padding: "8px 12px",
                borderRadius: "6px",
                background: "var(--bg-input)",
                border: "1px solid var(--bg-card-border)",
                color: "var(--text-primary)",
                fontSize: "13px",
                marginBottom: "10px",
              }}
            >
              {eligiblePlaybooks.map((p) => (
                <option key={p.name} value={p.name}>
                  {p.name} ({p.spec?.ident?.author}/{p.spec?.ident?.package}@{p.spec?.ident?.version})
                </option>
              ))}
            </select>
          </div>
        )}

        {/* Mode Toggle */}
        <div>
          <label
            style={{
              display: "block",
              marginBottom: "6px",
              fontWeight: 600,
              fontSize: "13px",
            }}
          >
            Replay Mode
          </label>
          <div style={{ display: "flex", gap: "0", borderRadius: "6px", overflow: "hidden", border: "1px solid var(--bg-card-border)", width: "fit-content" }}>
            <button
              onClick={() => setMode("timed")}
              disabled={isRunning || starting}
              style={{
                padding: "6px 16px",
                fontSize: "12px",
                fontWeight: 600,
                border: "none",
                cursor: isRunning || starting ? "not-allowed" : "pointer",
                background: mode === "timed" ? "var(--color-primary)" : "var(--bg-input)",
                color: mode === "timed" ? "#fff" : "var(--text-muted)",
                transition: "all 0.15s ease",
              }}
            >
              ⏱ Timed
            </button>
            <button
              onClick={() => setMode("parallel")}
              disabled={isRunning || starting}
              style={{
                padding: "6px 16px",
                fontSize: "12px",
                fontWeight: 600,
                border: "none",
                borderLeft: "1px solid var(--bg-card-border)",
                cursor: isRunning || starting ? "not-allowed" : "pointer",
                background: mode === "parallel" ? "var(--color-primary)" : "var(--bg-input)",
                color: mode === "parallel" ? "#fff" : "var(--text-muted)",
                transition: "all 0.15s ease",
              }}
            >
              ⚡ Parallel
            </button>
          </div>
        </div>
        {/* Folder Path */}
        <div>
          <label
            htmlFor="replay-folder-path"
            style={{
              display: "block",
              marginBottom: "6px",
              fontWeight: 600,
              fontSize: "13px",
            }}
          >
            Folder Path{" "}
            <span
              className="text-muted"
              style={{ fontSize: "11px", fontWeight: "normal" }}
            >
              (absolute path to a directory containing data files)
            </span>
          </label>
          <input
            id="replay-folder-path"
            type="text"
            value={folderPath}
            onChange={(e) => setFolderPath(e.target.value)}
            placeholder="/path/to/data/folder"
            disabled={isRunning || starting}
            style={{
              width: "100%",
              fontFamily: "var(--font-mono)",
              fontSize: "13px",
            }}
          />
        </div>

        {/* Rate Controls - shown in timed mode */}
        {mode === "timed" && (
          <div style={{ display: "flex", gap: "20px" }}>
            <div style={{ flex: 1 }}>
              <label
                htmlFor="replay-interval"
                style={{
                  display: "block",
                  marginBottom: "6px",
                  fontWeight: 600,
                  fontSize: "13px",
                }}
              >
                Interval{" "}
                <span
                  className="text-muted"
                  style={{ fontSize: "11px", fontWeight: "normal" }}
                >
                  (ms between rows)
                </span>
              </label>
              <input
                id="replay-interval"
                type="number"
                min={0}
                value={intervalMs}
                onChange={(e) =>
                  setIntervalMs(Math.max(0, parseInt(e.target.value) || 0))
                }
                disabled={isRunning || starting}
                style={{ width: "100%" }}
              />
            </div>
            <div style={{ flex: 1 }}>
              <label
                htmlFor="replay-wiggle"
                style={{
                  display: "block",
                  marginBottom: "6px",
                  fontWeight: 600,
                  fontSize: "13px",
                }}
              >
                Wiggle{" "}
                <span
                  className="text-muted"
                  style={{ fontSize: "11px", fontWeight: "normal" }}
                >
                  (± random jitter in ms)
                </span>
              </label>
              <input
                id="replay-wiggle"
                type="number"
                min={0}
                value={wiggleMs}
                onChange={(e) =>
                  setWiggleMs(Math.max(0, parseInt(e.target.value) || 0))
                }
                disabled={isRunning || starting}
                style={{ width: "100%" }}
              />
            </div>
          </div>
        )}

        {/* Concurrency Control - shown in parallel mode */}
        {mode === "parallel" && (
          <div>
            <label
              htmlFor="replay-concurrency"
              style={{
                display: "block",
                marginBottom: "6px",
                fontWeight: 600,
                fontSize: "13px",
              }}
            >
              Concurrency{" "}
              <span
                className="text-muted"
                style={{ fontSize: "11px", fontWeight: "normal" }}
              >
                (number of parallel jobs)
              </span>
            </label>
            <input
              id="replay-concurrency"
              type="number"
              min={1}
              max={256}
              value={concurrency}
              onChange={(e) =>
                setConcurrency(Math.max(1, Math.min(256, parseInt(e.target.value) || 1)))
              }
              disabled={isRunning || starting}
              style={{ width: "120px" }}
            />
          </div>
        )}
      </div>

      {/* Action Buttons */}
      <div
        style={{
          display: "flex",
          gap: "10px",
          alignItems: "center",
          marginBottom: "20px",
        }}
      >
        {!isRunning ? (
          <button
            onClick={handleStart}
            disabled={starting || !folderPath.trim()}
            className="btn btn-success"
            style={{ padding: "8px 20px" }}
          >
            {starting ? "Starting..." : "▶ Start Replay"}
          </button>
        ) : (
          <button
            onClick={handleStop}
            className="btn btn-danger"
            style={{ padding: "8px 20px" }}
          >
            ⏹ Stop Replay
          </button>
        )}

        {isRunning && mode === "timed" && (
          <span
            style={{
              fontSize: "13px",
              color: "var(--text-muted)",
              fontStyle: "italic",
            }}
          >
            {getEta()}
          </span>
        )}
      </div>

      {/* Progress Section */}
      {(isRunning || isCompleted) && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "12px",
            padding: "18px",
            backgroundColor: "rgba(255,255,255,0.01)",
            borderRadius: "8px",
            border: "1px solid var(--bg-card-border)",
          }}
        >
          {/* Status header */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              fontSize: "14px",
              fontWeight: 500,
            }}
          >
            <span
              style={{
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              {isRunning ? (
                <span
                  style={{
                    display: "inline-block",
                    width: "8px",
                    height: "8px",
                    borderRadius: "50%",
                    backgroundColor: "#10b981",
                    boxShadow: "0 0 6px #10b981",
                    animation: "pulse 1.5s infinite",
                  }}
                />
              ) : (
                <span
                  style={{
                    display: "inline-block",
                    width: "8px",
                    height: "8px",
                    borderRadius: "50%",
                    backgroundColor: "var(--text-muted)",
                  }}
                />
              )}
              {isRunning ? "Replaying..." : isCompleted ? "Replay Complete" : "Replay Stopped"}
            </span>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "13px" }}>
              {status.rows_completed} / {status.total_rows} rows ({progressPct}
              %)
            </span>
          </div>

          {/* Progress Bar */}
          <div
            style={{
              width: "100%",
              height: "8px",
              backgroundColor: "rgba(255,255,255,0.05)",
              borderRadius: "4px",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                width: `${progressPct}%`,
                height: "100%",
                backgroundColor:
                  status.errors > 0 && !isRunning
                    ? "var(--color-warning, #f59e0b)"
                    : "var(--color-primary)",
                boxShadow: `0 0 8px ${
                  status.errors > 0 && !isRunning
                    ? "var(--color-warning, #f59e0b)"
                    : "var(--color-primary)"
                }`,
                transition: "width 0.3s ease",
              }}
            />
          </div>

          {/* Stats */}
          <div style={{ display: "flex", gap: "15px", flexWrap: "wrap" }}>
            <span
              className="badge badge-online"
              style={{ fontSize: "11px", padding: "4px 10px" }}
            >
              ✓ Successes: {status.successes}
            </span>
            {status.errors > 0 && (
              <span
                className="badge badge-offline"
                style={{ fontSize: "11px", padding: "4px 10px" }}
              >
                ✗ Errors: {status.errors}
              </span>
            )}
            {status.current_file && (
              <span
                style={{
                  fontSize: "12px",
                  color: "var(--text-muted)",
                  fontFamily: "var(--font-mono)",
                  padding: "4px 0",
                }}
              >
                📄 {status.current_file}
              </span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
