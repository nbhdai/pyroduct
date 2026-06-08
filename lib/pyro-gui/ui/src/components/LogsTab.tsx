import { useState, useEffect, useRef } from "react";
import { LogEntry } from "../types";

interface LogsTabProps {
  logs: LogEntry[];
  onClearLogs: () => void;
}

export function LogsTab({ logs, onClearLogs }: LogsTabProps) {
  const [filter, setFilter] = useState<"all" | "error" | "success" | "command" | "system">("all");
  const consoleEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when logs change
  useEffect(() => {
    if (consoleEndRef.current) {
      consoleEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs]);

  const filteredLogs = logs.filter((log) => {
    if (filter === "all") return true;
    return log.type === filter;
  });

  const handleCopyAll = () => {
    const text = filteredLogs
      .map((log) => `[${log.time}] [${log.type.toUpperCase()}] ${log.message}`)
      .join("\n");
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="tab-content active" style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="repository-header" style={{ marginBottom: "20px", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h2 style={{ fontSize: "18px", fontWeight: "600", margin: 0 }}>System Event & Error Console</h2>
          <p className="subtitle" style={{ marginTop: "4px" }}>View, filter, and copy logs and error outputs.</p>
        </div>
        <div style={{ display: "flex", gap: "10px" }}>
          <button onClick={handleCopyAll} className="btn btn-primary">
            Copy Visible Logs
          </button>
          <button onClick={onClearLogs} className="btn btn-secondary">
            Clear Logs
          </button>
        </div>
      </div>

      {/* Filter Buttons */}
      <div className="tabs-sub" style={{ marginBottom: "20px" }}>
        <button className={`sub-tab-btn ${filter === "all" ? "active" : ""}`} onClick={() => setFilter("all")}>
          All ({logs.length})
        </button>
        <button className={`sub-tab-btn ${filter === "error" ? "active" : ""}`} onClick={() => setFilter("error")}>
          Errors ({logs.filter(l => l.type === "error").length})
        </button>
        <button className={`sub-tab-btn ${filter === "success" ? "active" : ""}`} onClick={() => setFilter("success")}>
          Success ({logs.filter(l => l.type === "success").length})
        </button>
        <button className={`sub-tab-btn ${filter === "command" ? "active" : ""}`} onClick={() => setFilter("command")}>
          Commands ({logs.filter(l => l.type === "command").length})
        </button>
        <button className={`sub-tab-btn ${filter === "system" ? "active" : ""}`} onClick={() => setFilter("system")}>
          System ({logs.filter(l => l.type === "system").length})
        </button>
      </div>

      {/* Console Display */}
      <div className="card" style={{ flexGrow: 1, display: "flex", flexDirection: "column", minHeight: 0, padding: "20px" }}>
        <div 
          className="console-box" 
          style={{ 
            flexGrow: 1, 
            maxHeight: "none", 
            overflowY: "auto", 
            userSelect: "text", 
            WebkitUserSelect: "text",
            fontSize: "14px",
            lineHeight: "1.7",
            padding: "16px"
          }}
        >
          {filteredLogs.length === 0 ? (
            <div style={{ color: "var(--text-muted)", textAlign: "center", padding: "40px" }}>
              No logs found matching filter.
            </div>
          ) : (
            filteredLogs.map((log, index) => (
              <div key={index} className={`log-line ${log.type}`} style={{ paddingBottom: "4px" }}>
                <span style={{ color: "var(--text-muted)", marginRight: "8px" }}>[{log.time}]</span>
                <span className={`badge badge-${log.type}`} style={{ fontSize: "10px", padding: "2px 6px", marginRight: "8px", verticalAlign: "middle" }}>
                  {log.type.toUpperCase()}
                </span>
                <span style={{ verticalAlign: "middle" }}>{log.message}</span>
              </div>
            ))
          )}
          <div ref={consoleEndRef} />
        </div>
      </div>
    </div>
  );
}
