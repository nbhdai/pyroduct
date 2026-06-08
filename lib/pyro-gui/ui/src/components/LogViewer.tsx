import { useState } from "react";

export interface LogRecord {
  module_logs: string[];
  capability_logs: Record<string, string[]>;
}

interface LogViewerProps {
  logs?: LogRecord | null;
}

const formatCapKey = (key: string): string => {
  try {
    const match = key.match(/"([^"]+)"/g);
    if (match && match.length >= 2) {
      const pkg = match[0].replace(/"/g, '');
      const ver = match[1].replace(/"/g, '');
      return `${pkg}@${ver}`;
    }
  } catch (e) {}
  return key;
};

export function LogViewer({ logs }: LogViewerProps) {
  const [activeTab, setActiveTab] = useState<"module" | string>("module");

  if (!logs) {
    return <div style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No logs available for this execution.</div>;
  }

  const hasCapabilityLogs = logs.capability_logs && Object.keys(logs.capability_logs).length > 0;
  const capabilityKeys = hasCapabilityLogs ? Object.keys(logs.capability_logs) : [];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "12px", width: "100%" }}>
      {/* Log Tabs */}
      <div className="tabs-sub" style={{ marginBottom: "10px", display: "flex", flexWrap: "wrap", gap: "8px" }}>
        <button
          className={`sub-tab-btn ${activeTab === "module" ? "active" : ""}`}
          onClick={() => setActiveTab("module")}
          style={{ fontSize: "13px", padding: "6px 12px" }}
        >
          Module Logs ({logs.module_logs.length})
        </button>
        {capabilityKeys.map((key) => (
          <button
            key={key}
            className={`sub-tab-btn ${activeTab === key ? "active" : ""}`}
            onClick={() => setActiveTab(key)}
            style={{ fontSize: "13px", padding: "6px 12px" }}
          >
            Cap: {formatCapKey(key)} ({logs.capability_logs[key].length})
          </button>
        ))}
      </div>

      {/* Log Console Box */}
      <div 
        className="console-box" 
        style={{ 
          maxHeight: "350px", 
          overflowY: "auto", 
          padding: "16px", 
          backgroundColor: "#0d0e12", 
          borderRadius: "8px",
          border: "1px solid var(--bg-card-border)"
        }}
      >
        {activeTab === "module" ? (
          logs.module_logs.length === 0 ? (
            <div style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No module log output.</div>
          ) : (
            logs.module_logs.map((line, idx) => (
              <div key={idx} className="log-line" style={{ whiteSpace: "pre-wrap", fontFamily: "monospace", fontSize: "13px", paddingBottom: "2px" }}>
                {line}
              </div>
            ))
          )
        ) : (
          logs.capability_logs[activeTab]?.length === 0 ? (
            <div style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No logs for this capability.</div>
          ) : (
            logs.capability_logs[activeTab]?.map((line, idx) => (
              <div key={idx} className="log-line" style={{ whiteSpace: "pre-wrap", fontFamily: "monospace", fontSize: "13px", paddingBottom: "2px" }}>
                {line}
              </div>
            ))
          )
        )}
      </div>
    </div>
  );
}
