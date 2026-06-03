import { useState } from "react";
import { CacheStatus } from "../types";

interface RepositoryTabProps {
  cacheStatus: CacheStatus | null;
  onPurgeCache: () => void;
}

export function RepositoryTab({ cacheStatus, onPurgeCache }: RepositoryTabProps) {
  const [subTab, setSubTab] = useState<"capabilities" | "modules">("capabilities");

  const capabilities = cacheStatus?.capabilities ?? [];
  const modules = cacheStatus?.modules ?? [];
  const cacheRoot = cacheStatus?.cache_root ?? "~/.pyroduct";

  return (
    <div className="tab-content active">
      <div className="repository-header">
        <div>
          <p className="subtitle">
            Local Repository: <span className="code-text">{cacheRoot}</span>
          </p>
        </div>
        <button onClick={onPurgeCache} className="btn btn-danger">
          Purge Cache
        </button>
      </div>

      <div className="tabs-sub">
        <button
          className={`sub-tab-btn ${subTab === "capabilities" ? "active" : ""}`}
          onClick={() => setSubTab("capabilities")}
        >
          Capabilities
        </button>
        <button
          className={`sub-tab-btn ${subTab === "modules" ? "active" : ""}`}
          onClick={() => setSubTab("modules")}
        >
          Modules (Playbooks)
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
                </tr>
              </thead>
              <tbody>
                {capabilities.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="text-center">
                      No capabilities found in repository.
                    </td>
                  </tr>
                ) : (
                  capabilities.map((cap, idx) => (
                    <tr key={idx}>
                      <td>{cap.author}</td>
                      <td>{cap.name}</td>
                      <td>
                        <span className="code-text">{cap.version}</span>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {subTab === "modules" && (
        <div className="sub-tab-content active">
          <div className="table-container">
            <table className="table">
              <thead>
                <tr>
                  <th>Author</th>
                  <th>Package</th>
                  <th>Version</th>
                </tr>
              </thead>
              <tbody>
                {modules.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="text-center">
                      No modules found in repository.
                    </td>
                  </tr>
                ) : (
                  modules.map((mod, idx) => (
                    <tr key={idx}>
                      <td>{mod.author}</td>
                      <td>{mod.name}</td>
                      <td>
                        <span className="code-text">{mod.version}</span>
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
