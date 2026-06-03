import { useEffect, useRef } from "react";
import { DaemonStatus, LogEntry } from "../types";

interface DashboardTabProps {
  daemonStatus: DaemonStatus;
  onQueryStatus: () => void;
  onPurgeCache: () => void;
  logs: LogEntry[];
}

export function DashboardTab({ daemonStatus, onQueryStatus, onPurgeCache, logs }: DashboardTabProps) {
  const consoleEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (consoleEndRef.current) {
      consoleEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs]);

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
      <div className="grid-layout">
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
            <button onClick={onPurgeCache} className="btn btn-danger btn-block">
              Purge Local Cache
            </button>
          </div>
        </div>
      </div>

      {/* Console / Logs */}
      <div className="card log-card mt-20">
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
  );
}
