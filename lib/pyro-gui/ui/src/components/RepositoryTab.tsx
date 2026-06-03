import { useState } from "react";
import { CacheStatus } from "../types";
import { CapabilitySpecView } from "./CapabilitySpecView";
import { PlaybookSpecView } from "./PlaybookSpecView";

interface RepositoryTabProps {
  cacheStatus: CacheStatus | null;
}

type SelectedItem = {
  author: string;
  name: string;
  version: string;
} | null;

export function RepositoryTab({ cacheStatus }: RepositoryTabProps) {
  const [subTab, setSubTab] = useState<"capabilities" | "playbooks">("capabilities");
  const [selectedCap, setSelectedCap] = useState<SelectedItem>(null);
  const [selectedPlaybook, setSelectedPlaybook] = useState<SelectedItem>(null);

  const capabilities = cacheStatus?.capabilities ?? [];
  const playbooks = cacheStatus?.playbooks ?? [];
  const cacheRoot = cacheStatus?.cache_root ?? "~/.pyroduct";

  if (selectedCap) {
    return (
      <CapabilitySpecView
        author={selectedCap.author}
        name={selectedCap.name}
        version={selectedCap.version}
        onBack={() => setSelectedCap(null)}
      />
    );
  }

  if (selectedPlaybook) {
    return (
      <PlaybookSpecView
        author={selectedPlaybook.author}
        name={selectedPlaybook.name}
        version={selectedPlaybook.version}
        onBack={() => setSelectedPlaybook(null)}
      />
    );
  }

  return (
    <div className="tab-content active">
      <div className="repository-header">
        <div>
          <p className="subtitle">
            Local Repository: <span className="code-text">{cacheRoot}</span>
          </p>
        </div>
      </div>

      <div className="tabs-sub">
        <button
          className={`sub-tab-btn ${subTab === "capabilities" ? "active" : ""}`}
          onClick={() => setSubTab("capabilities")}
        >
          Capabilities
        </button>
        <button
          className={`sub-tab-btn ${subTab === "playbooks" ? "active" : ""}`}
          onClick={() => setSubTab("playbooks")}
        >
          Playbooks
        </button>
      </div>

      {subTab === "capabilities" && (
        <div className="sub-tab-content active">
          <div className="table-container">
            <table className="table">
              <thead>
                <tr>
                  <th>Author</th>
                  <th>Package</th>
                  <th>Version</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {capabilities.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="text-center">
                      No capabilities found in repository.
                    </td>
                  </tr>
                ) : (
                  capabilities.map((cap, idx) => (
                    <tr
                      key={idx}
                      className="cursor-pointer"
                      onClick={() =>
                        setSelectedCap({ author: cap.author, name: cap.name, version: cap.version })
                      }
                      style={{ cursor: "pointer" }}
                    >
                      <td>{cap.author}</td>
                      <td>{cap.name}</td>
                      <td>
                        <span className="code-text">{cap.version}</span>
                      </td>
                      <td>
                        <button className="btn btn-secondary btn-sm" style={{ padding: "4px 8px", fontSize: "12px" }}>
                          View Spec
                        </button>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {subTab === "playbooks" && (
        <div className="sub-tab-content active">
          <div className="table-container">
            <table className="table">
              <thead>
                <tr>
                  <th>Author</th>
                  <th>Package</th>
                  <th>Version</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {playbooks.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="text-center">
                      No playbooks found in repository.
                    </td>
                  </tr>
                ) : (
                  playbooks.map((mod, idx) => (
                    <tr
                      key={idx}
                      className="cursor-pointer"
                      onClick={() =>
                        setSelectedPlaybook({
                          author: mod.author,
                          name: mod.name,
                          version: mod.version,
                        })
                      }
                      style={{ cursor: "pointer" }}
                    >
                      <td>{mod.author}</td>
                      <td>{mod.name}</td>
                      <td>
                        <span className="code-text">{mod.version}</span>
                      </td>
                      <td>
                        <button className="btn btn-secondary btn-sm" style={{ padding: "4px 8px", fontSize: "12px" }}>
                          View Spec
                        </button>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
