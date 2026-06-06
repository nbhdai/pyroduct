import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LogViewer } from "./LogViewer";
import { unpackExecutionRecord } from "./CallPlaybookForm";

const formatFailureError = (failureField: any): string => {
  if (!failureField) return "Unknown error";
  if (typeof failureField === "string") return failureField;
  if (failureField.Ok) {
    const okVal = failureField.Ok;
    let msg = okVal.message || "";
    if (okVal.file) {
      msg += ` (at ${okVal.file}:${okVal.line}:${okVal.column})`;
    }
    if (okVal.error) {
      msg += ` - Error: ${okVal.error}`;
    }
    return msg || JSON.stringify(okVal);
  }
  if (failureField.Err) {
    return String(failureField.Err);
  }
  return JSON.stringify(failureField);
};

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
  const [mode, setMode] = useState<"browse" | "query" | "failures">("browse");
  const [limit, setLimit] = useState<number>(50);
  const [offset, setOffset] = useState<number>(0);
  const [sqlQuery, setSqlQuery] = useState<string>("SELECT * FROM data LIMIT 10");
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<QueryResult | null>(null);
  const [failures, setFailures] = useState<any[]>([]);
  const [selectedRecord, setSelectedRecord] = useState<any | null>(null);
  const [loadingRecord, setLoadingRecord] = useState<boolean>(false);

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

  const handleFetchFailures = async () => {
    if (!playbookName) return;
    setLoading(true);
    setError(null);
    try {
      const res = (await invoke("get_playbook_failures", {
        playbookName,
      })) as any[];
      setFailures(res);
    } catch (err) {
      setError(String(err));
      setFailures([]);
    } finally {
      setLoading(false);
    }
  };

  const handleInspectLogs = async (index: number) => {
    if (!playbookName) return;
    setLoadingRecord(true);
    setSelectedRecord(null);
    try {
      const record = await invoke("get_playbook_execution_record", {
        playbookName,
        id: index,
      });
      setSelectedRecord(record);
    } catch (err) {
      alert("Failed to fetch logs for row: " + err);
    } finally {
      setLoadingRecord(false);
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
      } else if (mode === "failures") {
        handleFetchFailures();
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
        <button
          className={`sub-tab-btn ${mode === "failures" ? "active" : ""}`}
          onClick={() => {
            setMode("failures");
            setResults(null);
            setError(null);
          }}
          style={{ fontSize: "14px", padding: "6px 12px 10px 12px" }}
        >
          Failures Log
        </button>
      </div>

      {/* Actions and inputs for each mode */}
      {mode !== "failures" && (
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
      )}

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
        ) : mode === "failures" ? (
          <div style={{ display: "flex", flexDirection: "column", flexGrow: 1, minHeight: 0 }}>
            <div className="table-container" style={{ flexGrow: 1, overflow: "auto", maxHeight: "400px" }}>
              <table className="table" style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead style={{ position: "sticky", top: 0, zIndex: 1, backgroundColor: "#0f1115" }}>
                  <tr>
                    <th style={{ width: "100px", color: "var(--text-muted)" }}>Row ID</th>
                    <th>Input Details</th>
                    <th>Error Detail</th>
                    <th style={{ width: "100px", textAlign: "right" }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {failures.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="text-center" style={{ padding: "40px", color: "var(--text-muted)" }}>
                        No execution failures recorded.
                      </td>
                    </tr>
                  ) : (
                    failures.map((record, idx) => {
                      const details = unpackExecutionRecord(record);
                      if (!details) return null;
                      return (
                        <tr key={idx}>
                          <td style={{ color: "var(--text-muted)", fontWeight: "bold" }}>{details.row_index}</td>
                          <td>
                            <div style={{ maxHeight: "100px", overflowY: "auto" }}>
                              <code className="code-text" style={{ fontSize: "11px", whiteSpace: "pre-wrap" }}>
                                {JSON.stringify(details.input, null, 2)}
                              </code>
                            </div>
                          </td>
                          <td style={{ color: "var(--color-danger)" }}>
                            <div style={{ maxHeight: "100px", overflowY: "auto", fontSize: "13px", fontFamily: "monospace" }}>
                              {formatFailureError(details.failure)}
                            </div>
                          </td>
                          <td style={{ textAlign: "right" }}>
                            <button
                              className="btn btn-secondary"
                              style={{ padding: "4px 8px", fontSize: "11px", height: "auto" }}
                              onClick={() => handleInspectLogs(details.row_index)}
                            >
                              Logs
                            </button>
                          </td>
                        </tr>
                      );
                    })
                  )}
                </tbody>
              </table>
            </div>
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
                    {mode === "browse" && (
                      <th style={{ width: "80px", textAlign: "right" }}>Actions</th>
                    )}
                  </tr>
                </thead>
                <tbody>
                  {results.rows.length === 0 ? (
                    <tr>
                      <td colSpan={results.schema.length + (mode === "browse" ? 2 : 1)} className="text-center" style={{ padding: "40px", color: "var(--text-muted)" }}>
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
                        {mode === "browse" && (
                          <td style={{ textAlign: "right" }}>
                            <button
                              className="btn btn-secondary"
                              style={{ padding: "4px 8px", fontSize: "11px", height: "auto" }}
                              onClick={() => handleInspectLogs(offset + idx)}
                            >
                              Logs
                            </button>
                          </td>
                        )}
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

      {/* Log Inspection Modal */}
      {(selectedRecord || loadingRecord) && (
        <div className="modal-overlay active" onClick={() => setSelectedRecord(null)}>
          <div className="modal modal-lg" onClick={(e) => e.stopPropagation()} style={{ maxWidth: "800px" }}>
            <div className="modal-header">
              <h3>Execution Logs (Row #{unpackExecutionRecord(selectedRecord)?.row_index ?? ""})</h3>
              <button className="modal-close" onClick={() => setSelectedRecord(null)}>
                &times;
              </button>
            </div>
            <div className="modal-body" style={{ minHeight: "200px" }}>
              {loadingRecord ? (
                <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", padding: "40px" }}>
                  <div className="spinner"></div>
                  <p style={{ marginTop: "12px", color: "var(--text-muted)" }}>Loading execution logs...</p>
                </div>
              ) : selectedRecord ? (
                <div>
                  <div style={{ marginBottom: "15px", display: "flex", flexDirection: "column", gap: "8px" }}>
                    <div>
                      <strong style={{ fontSize: "12px", color: "var(--text-muted)" }}>Status: </strong>
                      {unpackExecutionRecord(selectedRecord)?.is_success ? (
                        <span style={{ color: "var(--color-success)", fontWeight: "bold" }}>Success</span>
                      ) : (
                        <span style={{ color: "var(--color-danger)", fontWeight: "bold" }}>Failure</span>
                      )}
                    </div>
                    {!unpackExecutionRecord(selectedRecord)?.is_success && (
                      <div style={{ backgroundColor: "var(--color-danger-glow)", padding: "10px", borderRadius: "6px", border: "1px solid rgba(255,51,68,0.15)" }}>
                        <strong style={{ fontSize: "12px", color: "var(--color-danger)" }}>Error: </strong>
                        <span style={{ color: "var(--text-main)", fontSize: "13px", fontFamily: "monospace" }}>
                          {formatFailureError(unpackExecutionRecord(selectedRecord)?.failure)}
                        </span>
                      </div>
                    )}
                  </div>
                  <LogViewer logs={unpackExecutionRecord(selectedRecord)?.logs} />
                </div>
              ) : (
                <div style={{ color: "var(--text-muted)" }}>No execution record found.</div>
              )}
            </div>
            <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end", marginTop: "15px", borderTop: "1px solid var(--bg-card-border)", paddingTop: "15px" }}>
              <button className="btn btn-secondary" onClick={() => setSelectedRecord(null)}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
