import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DataExplorerProps {
  playbookName: string;
  refreshTrigger?: number;
}

interface QueryResult {
  schema: Array<{
    name: string;
    type: string;
    nullable: boolean;
  }>;
  rows: Array<Record<string, any>>;
}

export function DataExplorer({ playbookName, refreshTrigger }: DataExplorerProps) {
  const [mode, setMode] = useState<"browse" | "query">("browse");
  const [limit, setLimit] = useState<number>(50);
  const [offset, setOffset] = useState<number>(0);
  const [sqlQuery, setSqlQuery] = useState<string>("SELECT * FROM data LIMIT 10");
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<QueryResult | null>(null);

  const handleFetchBrowse = async (newOffset: number = offset) => {
    if (!playbookName) return;
    setLoading(true);
    setError(null);
    try {
      const res = (await invoke("get_playbook_data", {
        playbookName,
        offset: newOffset,
        limit,
      })) as QueryResult;
      setResults(res);
      setOffset(newOffset);
    } catch (err) {
      setError(String(err));
      setResults(null);
    } finally {
      setLoading(false);
    }
  };

  const handleExecuteQuery = async () => {
    if (!playbookName) return;
    setLoading(true);
    setError(null);
    try {
      const res = (await invoke("query_playbook_data", {
        playbookName,
        sqlQuery,
      })) as QueryResult;
      setResults(res);
    } catch (err) {
      setError(String(err));
      setResults(null);
    } finally {
      setLoading(false);
    }
  };

  // Re-fetch data whenever playbookName changes, refreshTrigger increments, or limit changes
  useEffect(() => {
    setOffset(0);
    setResults(null);
    setError(null);
    if (playbookName) {
      if (mode === "browse") {
        handleFetchBrowse(0);
      } else {
        handleExecuteQuery();
      }
    }
  }, [playbookName, refreshTrigger, mode]);

  const renderCellValue = (val: any) => {
    if (val === null || val === undefined) {
      return <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>∅</span>;
    }
    if (typeof val === "object") {
      return <code className="code-text" style={{ fontSize: "11px", whiteSpace: "pre-wrap" }}>{JSON.stringify(val)}</code>;
    }
    if (typeof val === "boolean") {
      return val ? "true" : "false";
    }
    return String(val);
  };

  const isPrevDisabled = offset === 0 || loading;
  const isNextDisabled = !results || results.rows.length < limit || loading;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", marginTop: "10px" }}>
      {/* Sub Tabs */}
      <div className="tabs-sub" style={{ marginBottom: "15px" }}>
        <button
          className={`sub-tab-btn ${mode === "browse" ? "active" : ""}`}
          onClick={() => {
            setMode("browse");
            setResults(null);
            setError(null);
          }}
          style={{ fontSize: "14px", padding: "6px 12px 10px 12px" }}
        >
          Table Browser
        </button>
        <button
          className={`sub-tab-btn ${mode === "query" ? "active" : ""}`}
          onClick={() => {
            setMode("query");
            setResults(null);
            setError(null);
          }}
          style={{ fontSize: "14px", padding: "6px 12px 10px 12px" }}
        >
          SQL Console
        </button>
      </div>

      {/* Actions and inputs for each mode */}
      <div className="card" style={{ marginBottom: "20px", padding: "20px" }}>
        {mode === "browse" ? (
          <div style={{ display: "flex", alignItems: "flex-end", gap: "16px", flexWrap: "wrap" }}>
            <div className="form-group" style={{ margin: 0, width: "120px" }}>
              <label style={{ fontSize: "12px", marginBottom: "4px" }}>Limit</label>
              <input
                type="number"
                value={limit}
                onChange={(e) => setLimit(Math.max(1, parseInt(e.target.value) || 10))}
                style={{
                  width: "100%",
                  padding: "8px 12px",
                  backgroundColor: "rgba(255, 255, 255, 0.03)",
                  border: "1px solid var(--bg-card-border)",
                  borderRadius: "8px",
                  color: "var(--text-main)",
                }}
              />
            </div>
            <div className="form-group" style={{ margin: 0, width: "120px" }}>
              <label style={{ fontSize: "12px", marginBottom: "4px" }}>Offset</label>
              <input
                type="number"
                value={offset}
                onChange={(e) => setOffset(Math.max(0, parseInt(e.target.value) || 0))}
                style={{
                  width: "100%",
                  padding: "8px 12px",
                  backgroundColor: "rgba(255, 255, 255, 0.03)",
                  border: "1px solid var(--bg-card-border)",
                  borderRadius: "8px",
                  color: "var(--text-main)",
                }}
              />
            </div>
            <button
              onClick={() => handleFetchBrowse(offset)}
              className="btn btn-primary"
              disabled={loading || !playbookName}
              style={{ height: "38px", padding: "0 20px" }}
            >
              Fetch Data
            </button>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <div className="form-group" style={{ margin: 0 }}>
              <label style={{ fontSize: "12px", marginBottom: "4px" }}>Execute SQL against table 'data'</label>
              <textarea
                value={sqlQuery}
                onChange={(e) => setSqlQuery(e.target.value)}
                rows={3}
                placeholder="e.g. SELECT * FROM data LIMIT 10"
                style={{
                  width: "100%",
                  padding: "12px",
                  backgroundColor: "rgba(255, 255, 255, 0.03)",
                  border: "1px solid var(--bg-card-border)",
                  borderRadius: "8px",
                  color: "var(--text-main)",
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: "13px",
                }}
              />
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button
                onClick={handleExecuteQuery}
                className="btn btn-primary"
                disabled={loading || !playbookName}
                style={{ padding: "8px 20px" }}
              >
                Execute Query
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Error display */}
      {error && (
        <div
          className="log-line error"
          style={{
            backgroundColor: "var(--color-danger-glow)",
            border: "1px solid rgba(255, 51, 68, 0.2)",
            borderRadius: "8px",
            padding: "12px 16px",
            color: "var(--color-danger)",
            marginBottom: "20px",
            fontSize: "14px",
            fontFamily: "monospace",
            whiteSpace: "pre-wrap",
          }}
        >
          <strong>Error:</strong> {error}
        </div>
      )}

      {/* Results grid */}
      <div style={{ flexGrow: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
        {loading ? (
          <div className="spec-loading" style={{ padding: "40px 0", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center" }}>
            <div className="spinner"></div>
            <p style={{ marginTop: "16px", color: "var(--text-muted)", fontSize: "14px" }}>Querying daemon data store...</p>
          </div>
        ) : results ? (
          <div style={{ display: "flex", flexDirection: "column", flexGrow: 1, minHeight: 0 }}>
            <div className="table-container" style={{ flexGrow: 1, overflow: "auto", maxHeight: "400px" }}>
              <table className="table" style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead style={{ position: "sticky", top: 0, zIndex: 1, backgroundColor: "#0f1115" }}>
                  <tr>
                    <th style={{ width: "60px", color: "var(--text-muted)" }}>#</th>
                    {results.schema.map((field) => (
                      <th key={field.name} title={`${field.type}${field.nullable ? ' (nullable)' : ''}`}>
                        {field.name}
                        <span style={{ display: "block", fontSize: "10px", fontWeight: "normal", color: "var(--text-muted)" }}>
                          {field.type}
                        </span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {results.rows.length === 0 ? (
                    <tr>
                      <td colSpan={results.schema.length + 1} className="text-center" style={{ padding: "40px", color: "var(--text-muted)" }}>
                        No records returned.
                      </td>
                    </tr>
                  ) : (
                    results.rows.map((row, idx) => (
                      <tr key={idx}>
                        <td style={{ color: "var(--text-muted)" }}>{offset + idx}</td>
                        {results.schema.map((field) => (
                          <td key={field.name}>{renderCellValue(row[field.name])}</td>
                        ))}
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>

            {/* Pagination for Browse mode */}
            {mode === "browse" && (
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  padding: "16px 0 0 0",
                  borderTop: "1px solid var(--bg-card-border)",
                  marginTop: "16px",
                }}
              >
                <div style={{ color: "var(--text-muted)", fontSize: "13px" }}>
                  Showing rows {offset} - {offset + (results?.rows.length ?? 0)}
                </div>
                <div style={{ display: "flex", gap: "10px" }}>
                  <button
                    className="btn btn-secondary"
                    disabled={isPrevDisabled}
                    onClick={() => handleFetchBrowse(Math.max(0, offset - limit))}
                    style={{ padding: "6px 12px", fontSize: "13px" }}
                  >
                    Previous Page
                  </button>
                  <button
                    className="btn btn-secondary"
                    disabled={isNextDisabled}
                    onClick={() => handleFetchBrowse(offset + limit)}
                    style={{ padding: "6px 12px", fontSize: "13px" }}
                  >
                    Next Page
                  </button>
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="empty-state" style={{ padding: "40px", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center" }}>
            <div className="empty-icon">📊</div>
            <p>Fetch Data or Execute Query to inspect records.</p>
          </div>
        )}
      </div>
    </div>
  );
}
