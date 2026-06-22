import { Playbook } from "../types";
import { PlaybookDetailView } from "./PlaybookDetailView";

interface PlaybooksTabProps {
  playbooks: Playbook[];
  onStartPlaybookClick: () => void;
  onStopPlaybookClick: (name: string) => void;
  onSubmitCall: (name: string, payload: any) => Promise<any>;
  selectedPlaybookName: string | null;
  setSelectedPlaybookName: (name: string | null) => void;
}

export function PlaybooksTab({
  playbooks,
  onStartPlaybookClick,
  onStopPlaybookClick,
  onSubmitCall,
  selectedPlaybookName,
  setSelectedPlaybookName,
}: PlaybooksTabProps) {
  // If a playbook is selected, render the detail view inline instead of the list grid
  if (selectedPlaybookName !== null) {
    const selectedPlaybook = playbooks.find((pb) => pb.name === selectedPlaybookName);
    if (selectedPlaybook) {
      return (
        <div className="tab-content active">
          <PlaybookDetailView
            playbook={selectedPlaybook}
            playbooks={playbooks}
            onBack={() => setSelectedPlaybookName(null)}
            onSubmitCall={onSubmitCall}
          />
        </div>
      );
    }
  }

  return (
    <div className="tab-content active">
      <div className="playbooks-header">
        <h2>Running Workers</h2>
        <button onClick={onStartPlaybookClick} className="btn btn-success">
          + Start Playbook
        </button>
      </div>

      <div className="playbooks-grid">
        {playbooks.length === 0 ? (
          <div className="empty-state">
            <span className="empty-icon">⚙</span>
            <p>No active playbooks found running on the daemon.</p>
          </div>
        ) : (
          playbooks.map((pb, idx) => {
            const capabilities = pb.active_capabilities ?? [];
            return (
              <div key={idx} className="playbook-card">
                <div className="playbook-card-header">
                  <h3>{pb.name}</h3>
                  <span className="badge badge-online">Running</span>
                </div>
                <div className="playbook-details">
                  <div className="detail-item">
                    <span className="lbl">Config Path</span>
                    <span className="val code-text">{pb.config_path}</span>
                  </div>
                  <div className="detail-item">
                    <span className="lbl">HTTP Address</span>
                    <span className="val code-text">{pb.http_address || "None"}</span>
                  </div>
                  <div className="detail-item">
                    <span className="lbl">Active Capabilities</span>
                    <div className="capability-pills">
                      {capabilities.length === 0 ? (
                        <span className="text-muted" style={{ fontSize: "11px" }}>
                          None
                        </span>
                      ) : (
                        capabilities.map((cap, cIdx) => (
                          <span key={cIdx} className="cap-pill">
                            {cap.package}@{cap.version}
                          </span>
                        ))
                      )}
                    </div>
                  </div>
                </div>
                <div className="playbook-actions">
                  <button
                    onClick={() => setSelectedPlaybookName(pb.name)}
                    className="btn btn-secondary call-pb-btn"
                  >
                    Call
                  </button>
                  <button
                    onClick={() => onStopPlaybookClick(pb.name)}
                    className="btn btn-danger stop-pb-btn"
                  >
                    Stop
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
