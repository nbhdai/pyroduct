import React, { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface BulkUploadFormProps {
  playbookName: string;
  onSuccess?: () => void;
}

export function BulkUploadForm({ playbookName, onSuccess }: BulkUploadFormProps) {
  const [file, setFile] = useState<File | null>(null);
  const [processing, setProcessing] = useState(false);
  const [processedCount, setProcessedCount] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [successCount, setSuccessCount] = useState(0);
  const [failureCount, setFailureCount] = useState(0);
  const [results, setResults] = useState<any[]>([]);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFile = e.target.files?.[0];
    if (!selectedFile) return;

    setFile(selectedFile);
    setErrorMsg(null);
    setResults([]);
    setProcessedCount(0);
    setTotalCount(0);
    setSuccessCount(0);
    setFailureCount(0);
  };

  const startProcessing = async () => {
    if (!file || processing) return;

    setProcessing(true);
    setErrorMsg(null);
    setProcessedCount(0);
    setTotalCount(0);
    setSuccessCount(0);
    setFailureCount(0);
    setResults([]);

    try {
      // 1. Read file bytes on frontend
      const arrayBuffer = await file.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);
      const bytes = Array.from(uint8Array);

      // 2. Send bytes directly to daemon for bulk evaluation using Rowable Arrow reader
      const daemonResults = (await invoke("run_bulk_playbook", {
        playbookName,
        fileName: file.name,
        fileContent: bytes,
      })) as any[];

      // 3. Map daemon ServerExecutionRecords into the UI results format
      const runResults = daemonResults.map((res: any, idx: number) => {
        const inner = res.Normal || res.Session || res.SessionDiff;
        if (!inner) {
          return {
            row_index: idx,
            input: {},
            success: false,
            error: "Malformed execution record returned from daemon",
          };
        }

        if (inner.Success) {
          if (onSuccess) onSuccess();
          return {
            row_index: inner.Success.row_index ?? idx,
            input: inner.Success.input || {},
            success: true,
            output: inner.Success.success || {},
          };
        } else if (inner.Failure) {
          const errorMsg = typeof inner.Failure.failure === "string" 
            ? inner.Failure.failure 
            : (inner.Failure.failure?.Ok?.message || JSON.stringify(inner.Failure.failure) || "Row execution failed");
          return {
            row_index: inner.Failure.row_index ?? idx,
            input: inner.Failure.input || {},
            success: false,
            error: errorMsg,
          };
        }

        return {
          row_index: idx,
          input: {},
          success: false,
          error: "Unknown execution record variant",
        };
      });

      // 4. Update UI counts
      let successes = 0;
      let failures = 0;
      runResults.forEach((r) => {
        if (r.success) successes++;
        else failures++;
      });

      setSuccessCount(successes);
      setFailureCount(failures);
      setTotalCount(runResults.length);
      setProcessedCount(runResults.length);
      setResults(runResults);

    } catch (err: any) {
      console.error(err);
      setErrorMsg(`Failed to run bulk playbook evaluation: ${err}`);
    } finally {
      setProcessing(false);
    }
  };

  const downloadResultsJSON = () => {
    const content = JSON.stringify(results, null, 2);
    const blob = new Blob([content], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${playbookName}_bulk_results.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const downloadResultsCSV = () => {
    if (results.length === 0) return;
    
    // Find all keys from input and output to build flat headers
    const headers = new Set<string>();
    headers.add("row_index");
    headers.add("status");
    headers.add("error");

    results.forEach((r) => {
      if (r.input && typeof r.input === "object") {
        Object.keys(r.input).forEach((k) => headers.add(`input_${k}`));
      }
      if (r.output && typeof r.output === "object") {
        Object.keys(r.output).forEach((k) => headers.add(`output_${k}`));
      } else if (r.output !== undefined && r.output !== null) {
        headers.add("output_value");
      }
    });

    const headerArray = Array.from(headers);
    const csvRows = [headerArray.join(",")];

    results.forEach((r) => {
      const rowValues = headerArray.map((header) => {
        if (header === "row_index") return r.row_index;
        if (header === "status") return r.success ? "success" : "failure";
        if (header === "error") return `"${(r.error || "").replace(/"/g, '""')}"`;
        
        if (header.startsWith("input_")) {
          const key = header.substring(6);
          const val = r.input?.[key];
          return `"${String(val !== undefined && val !== null ? val : "").replace(/"/g, '""')}"`;
        }
        
        if (header.startsWith("output_")) {
          const key = header.substring(7);
          if (key === "value") {
            return `"${String(r.output !== undefined && r.output !== null ? r.output : "").replace(/"/g, '""')}"`;
          }
          const val = r.output?.[key];
          return `"${String(val !== undefined && val !== null ? val : "").replace(/"/g, '""')}"`;
        }

        return "";
      });
      csvRows.push(rowValues.join(","));
    });

    const blob = new Blob([csvRows.join("\n")], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${playbookName}_bulk_results.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const progressPct = totalCount > 0 ? Math.round((processedCount / totalCount) * 100) : (processing ? 50 : 0);

  return (
    <div className="card" style={{ marginBottom: "20px" }}>
      <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "15px" }}>
        Bulk Processing Executor
      </h3>
      <p className="text-muted text-sm mt-5 mb-20">
        Upload a JSON, CSV, JSONL, Parquet, or Arrow IPC file to run bulk playbook evaluations on the daemon using Arrow.
      </p>

      {/* File Selector */}
      <div style={{ display: "flex", alignItems: "center", gap: "15px", marginBottom: "20px" }}>
        <input
          type="file"
          ref={fileInputRef}
          onChange={handleFileChange}
          accept=".csv,.json,.jsonl,.parquet,.arrow,.ipc"
          style={{ display: "none" }}
        />
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          className="btn btn-secondary"
          disabled={processing}
        >
          📁 Select Data File
        </button>
        {file && (
          <span style={{ fontSize: "14px", fontWeight: 500 }}>
            {file.name} ({(file.size / 1024).toFixed(1)} KB)
          </span>
        )}
      </div>

      {errorMsg && (
        <div 
          className="card p-12 mb-15 text-sm" 
          style={{ 
            borderRadius: "8px", 
            border: "1px solid var(--color-danger)", 
            backgroundColor: "var(--color-danger-glow)",
            color: "var(--color-danger)" 
          }}
        >
          {errorMsg}
        </div>
      )}

      {/* Dataset Preview */}
      {file && (
        <div style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
          <div 
            style={{ 
              display: "flex", 
              justifyContent: "space-between", 
              alignItems: "center",
              padding: "10px 14px",
              backgroundColor: "rgba(255,255,255,0.02)",
              borderRadius: "8px",
              border: "1px solid var(--bg-card-border)"
            }}
          >
            <span style={{ fontSize: "14px", fontWeight: 600 }}>
              Dataset Target: <strong style={{ color: "var(--color-primary)" }}>{file.name}</strong> ready
            </span>
            {!processing && results.length === 0 && (
              <button onClick={startProcessing} className="btn btn-success" style={{ padding: "6px 16px" }}>
                🚀 Run Bulk Executor
              </button>
            )}
          </div>

          {/* Progress Section */}
          {(processing || results.length > 0) && (
            <div style={{
              display: "flex", 
              flexDirection: "column", 
              gap: "10px",
              padding: "16px",
              backgroundColor: "rgba(255,255,255,0.01)",
              borderRadius: "8px",
              border: "1px solid var(--bg-card-border)"
            }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "14px", fontWeight: 500 }}>
                <span>Processing Status</span>
                <span>{processing ? "Evaluating dataset on daemon..." : `Completed processing: ${results.length} rows`}</span>
              </div>
              
              {/* Progress Bar */}
              <div style={{ width: "100%", height: "8px", backgroundColor: "rgba(255,255,255,0.05)", borderRadius: "4px", overflow: "hidden" }}>
                <div style={{
                  width: `${progressPct}%`,
                  height: "100%",
                  backgroundColor: "var(--color-primary)",
                  boxShadow: "0 0 8px var(--color-primary)",
                  transition: "width 0.3s ease"
                }} />
              </div>

              {/* Status Counter Badges */}
              {!processing && results.length > 0 && (
                <div style={{ display: "flex", gap: "15px", marginTop: "5px" }}>
                  <span className="badge badge-online" style={{ fontSize: "11px", padding: "4px 8px" }}>
                    Successes: {successCount}
                  </span>
                  <span className="badge badge-offline" style={{ fontSize: "11px", padding: "4px 8px" }}>
                    Failures: {failureCount}
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Execution Results Summary */}
          {!processing && results.length > 0 && (
            <div style={{ display: "flex", flexDirection: "column", gap: "12px", marginTop: "10px" }}>
              <div style={{ display: "flex", gap: "10px" }}>
                <button onClick={downloadResultsJSON} className="btn btn-primary" style={{ padding: "8px 16px" }}>
                  📥 Download JSON Results
                </button>
                <button onClick={downloadResultsCSV} className="btn btn-secondary" style={{ padding: "8px 16px" }}>
                  📥 Download CSV Results
                </button>
              </div>

              {/* Preview Grid */}
              <div style={{ maxHeight: "250px", overflowY: "auto", border: "1px solid var(--bg-card-border)", borderRadius: "8px" }}>
                <table className="table" style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr style={{ backgroundColor: "rgba(255,255,255,0.02)" }}>
                      <th style={{ padding: "10px" }}>Row</th>
                      <th style={{ padding: "10px" }}>Input Preview</th>
                      <th style={{ padding: "10px" }}>Status</th>
                      <th style={{ padding: "10px" }}>Output/Error Detail</th>
                    </tr>
                  </thead>
                  <tbody>
                    {results.slice(0, 50).map((r, i) => (
                      <tr key={i}>
                        <td style={{ padding: "10px", fontWeight: "bold" }}>#{i + 1}</td>
                        <td style={{ padding: "10px", fontFamily: "monospace", fontSize: "12px" }}>
                          {JSON.stringify(r.input).substring(0, 60)}...
                        </td>
                        <td style={{ padding: "10px" }}>
                          <span className={r.success ? "badge badge-online" : "badge badge-offline"} style={{ fontSize: "10px", padding: "2px 6px" }}>
                            {r.success ? "Success" : "Failure"}
                          </span>
                        </td>
                        <td style={{ 
                          padding: "10px", 
                          fontFamily: "monospace", 
                          fontSize: "12px",
                          color: r.success ? "var(--color-success)" : "var(--color-danger)"
                        }}>
                          {r.success 
                            ? JSON.stringify(r.output).substring(0, 80)
                            : (r.error || "Unknown error")}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {results.length > 50 && (
                  <div style={{ textAlign: "center", padding: "12px", color: "var(--text-muted)", fontSize: "13px", borderTop: "1px solid var(--bg-card-border)" }}>
                    Showing first 50 results. Download full dataset results using the buttons above.
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
