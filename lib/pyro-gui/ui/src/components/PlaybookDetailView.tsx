import { useState } from "react";
import { Playbook } from "../types";
import { CallPlaybookForm } from "./CallPlaybookForm";
import { PlaybookChat } from "./PlaybookChat";
import { DataExplorer } from "./DataExplorer";

function isChatPlaybook(playbook: Playbook): boolean {
  if (!playbook.spec || !playbook.spec.func) return false;
  const func = playbook.spec.func;
  
  if (func.kind !== "session" && func.kind !== "session_diff") return false;
  
  const inputField = func.input?.fields?.find((f: any) => f.name === "input");
  if (!inputField) return false;
  
  const dt = inputField.data_type;
  if (!dt || typeof dt !== "object" || !dt.Group) return false;
  
  const groupFields = dt.Group;
  if (groupFields.length !== 2) return false;
  
  const roleField = groupFields.find((f: any) => f.name === "role");
  const contentField = groupFields.find((f: any) => f.name === "content");
  
  if (!roleField || roleField.data_type !== "Str") return false;
  if (!contentField || contentField.data_type !== "Str") return false;
  
  return true;
}

interface PlaybookDetailViewProps {
  playbook: Playbook;
  onBack: () => void;
  onSubmitCall: (name: string, payload: any, sessionId?: number) => Promise<any>;
}

export function PlaybookDetailView({ playbook, onBack, onSubmitCall }: PlaybookDetailViewProps) {
  const [refreshTrigger, setRefreshTrigger] = useState<number>(0);
  const [prevPlaybookName, setPrevPlaybookName] = useState<string>(playbook.name);
  const chatCompatible = isChatPlaybook(playbook);
  const [activeSubTab, setActiveSubTab] = useState<"chat" | "form">(chatCompatible ? "chat" : "form");

  if (playbook.name !== prevPlaybookName) {
    setPrevPlaybookName(playbook.name);
    setActiveSubTab(chatCompatible ? "chat" : "form");
  }

  const handleCallSuccess = () => {
    // Increment refreshTrigger to signal DataExplorer to reload its tables
    setRefreshTrigger((prev) => prev + 1);
  };


  return (
    <div className="spec-view-container" style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      {/* Detail Header */}
      <div className="spec-view-header" style={{ marginBottom: "10px" }}>
        <div className="flex items-center gap-15" style={{ display: "flex", alignItems: "center", gap: "15px" }}>
          <button onClick={onBack} className="btn btn-secondary btn-sm btn-back" style={{ padding: "6px 12px", fontSize: "13px" }}>
            ← Back to Workers
          </button>
          <div>
            <span className="badge badge-online">Running Worker</span>
            <h2 className="spec-title mt-5" style={{ fontSize: "22px", margin: "5px 0 0 0" }}>
              {playbook.name}
            </h2>
          </div>
        </div>
      </div>

      {/* Playbook Worker Details */}
      <div className="card" style={{ padding: "18px 24px" }}>
        <h3 style={{ fontSize: "15px", fontWeight: 600, marginBottom: "12px" }}>Worker Metadata</h3>
        <div className="info-list" style={{ gap: "10px" }}>
          <div className="info-row" style={{ paddingBottom: "8px" }}>
            <span className="label">Config Path</span>
            <span className="value code-text">{playbook.config_path}</span>
          </div>
          <div className="info-row" style={{ paddingBottom: "8px" }}>
            <span className="label">Socket Path</span>
            <span className="value code-text">{playbook.socket_path || "None"}</span>
          </div>
          <div className="info-row" style={{ paddingBottom: "8px" }}>
            <span className="label">Local Capabilities</span>
            <div className="capability-pills" style={{ margin: 0 }}>
              {(!playbook.local_capabilities || playbook.local_capabilities.length === 0) ? (
                <span className="text-muted" style={{ fontSize: "12px" }}>
                  None
                </span>
              ) : (
                playbook.local_capabilities.map((cap, idx) => (
                  <span key={idx} className="cap-pill-local">
                    {cap.package}@{cap.version}
                  </span>
                ))
              )}
            </div>
          </div>
          <div className="info-row" style={{ paddingBottom: "0", borderBottom: "none" }}>
            <span className="label">Remote Capabilities</span>
            <div className="capability-pills" style={{ margin: 0 }}>
              {(!playbook.remote_capabilities || playbook.remote_capabilities.length === 0) ? (
                <span className="text-muted" style={{ fontSize: "12px" }}>
                  None
                </span>
              ) : (
                playbook.remote_capabilities.map((cap, idx) => (
                  <span key={idx} className="cap-pill">
                    {cap.package}@{cap.version}
                  </span>
                ))
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Sub Tab Switcher if chat compatible */}
      {chatCompatible && (
        <div style={{ display: "flex", gap: "10px", borderBottom: "1px solid var(--bg-card-border)", paddingBottom: "4px" }}>
          <button
            onClick={() => setActiveSubTab("chat")}
            className={`sub-tab-btn ${activeSubTab === "chat" ? "active" : ""}`}
            style={{
              background: "none",
              border: "none",
              color: activeSubTab === "chat" ? "var(--color-primary)" : "var(--text-muted)",
              fontSize: "14px",
              fontWeight: 600,
              padding: "8px 16px 12px 16px",
              cursor: "pointer",
              position: "relative"
            }}
          >
            💬 Chat
            {activeSubTab === "chat" && (
              <span style={{
                position: "absolute",
                bottom: "-5px",
                left: 0,
                right: 0,
                height: "2px",
                backgroundColor: "var(--color-primary)",
                boxShadow: "0 0 8px var(--color-primary)"
              }} />
            )}
          </button>
          <button
            onClick={() => setActiveSubTab("form")}
            className={`sub-tab-btn ${activeSubTab === "form" ? "active" : ""}`}
            style={{
              background: "none",
              border: "none",
              color: activeSubTab === "form" ? "var(--color-primary)" : "var(--text-muted)",
              fontSize: "14px",
              fontWeight: 600,
              padding: "8px 16px 12px 16px",
              cursor: "pointer",
              position: "relative"
            }}
          >
            ⚙️ Execute Form
            {activeSubTab === "form" && (
              <span style={{
                position: "absolute",
                bottom: "-5px",
                left: 0,
                right: 0,
                height: "2px",
                backgroundColor: "var(--color-primary)",
                boxShadow: "0 0 8px var(--color-primary)"
              }} />
            )}
          </button>
        </div>
      )}

      {/* Top Section: Chat or Form */}
      {activeSubTab === "chat" ? (
        <PlaybookChat
          playbookName={playbook.name}
          playbookSpec={playbook.spec}
          onSubmit={onSubmitCall}
        />
      ) : (
        <CallPlaybookForm
          playbookName={playbook.name}
          playbookSpec={playbook.spec}
          onSubmit={onSubmitCall}
          onSuccess={handleCallSuccess}
        />
      )}

      {/* Bottom Section: Data Explorer */}
      <div className="card" style={{ padding: "24px" }}>
        <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "15px" }}>
          Database Explorer
        </h3>
        <DataExplorer
          playbookName={playbook.name}
          refreshTrigger={refreshTrigger}
        />
      </div>
    </div>
  );
}
