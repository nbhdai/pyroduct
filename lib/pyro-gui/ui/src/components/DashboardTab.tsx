import { Playbook } from "../types";

interface DashboardTabProps {
  playbooks: Playbook[];
  onViewPlaybook: (name: string) => void;
}

export function DashboardTab({ playbooks, onViewPlaybook }: DashboardTabProps) {
  return (
    <div className="tab-content active">
      {/* Running Workers Card */}
      <div className="card">
        <h2>Active Workers ({playbooks.length})</h2>
        {playbooks.length === 0 ? (
          <div className="empty-state" style={{ padding: "30px 20px" }}>
            <span className="empty-icon" style={{ fontSize: "36px" }}>⚙</span>
            <p>No active playbook workers running.</p>
          </div>
        ) : (
          <div className="table-container mt-10">
            <table className="table">
              <thead>
                <tr>
                  <th>Worker Name</th>
                  <th>Rows Processed</th>
                  <th>HTTP Address</th>
                  <th>Capabilities</th>
                </tr>
              </thead>
              <tbody>
                {playbooks.map((pb, idx) => (
                  <tr key={idx}>
                    <td className="font-semibold text-primary">
                      <button
                        onClick={() => onViewPlaybook(pb.name)}
                        style={{
                          background: "none",
                          border: "none",
                          color: "var(--color-primary)",
                          fontFamily: "inherit",
                          fontSize: "inherit",
                          fontWeight: "inherit",
                          cursor: "pointer",
                          padding: 0,
                          textAlign: "left",
                          display: "inline-block",
                        }}
                        onMouseOver={(e) => (e.currentTarget.style.textDecoration = "underline")}
                        onMouseOut={(e) => (e.currentTarget.style.textDecoration = "none")}
                      >
                        {pb.name}
                      </button>
                    </td>
                    <td style={{ fontWeight: 600, color: "var(--color-success)" }}>
                      {pb.processed_rows ?? 0}
                    </td>
                    <td><span className="code-text">{pb.http_address || "None"}</span></td>
                    <td>
                      <div className="capability-pills">
                        {(pb.active_capabilities ?? []).length === 0 ? (
                          <span className="text-muted text-xs">None</span>
                        ) : (
                          (pb.active_capabilities ?? []).map((cap, cIdx) => (
                            <span key={cIdx} className="cap-pill" style={{ padding: "3px 6px", fontSize: "10px" }}>
                              {cap.package}
                            </span>
                          ))
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
