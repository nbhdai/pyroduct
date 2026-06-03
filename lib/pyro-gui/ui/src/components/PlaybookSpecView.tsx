import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PyroSchema } from "../types";

interface PlaybookSpecViewProps {
  author: string;
  name: string;
  version: string;
  onBack?: () => void;
}

export function PlaybookSpecView({ author, name, version, onBack }: PlaybookSpecViewProps) {
  const [spec, setSpec] = useState<any | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);

    invoke("get_playbook_spec", { author, name, version })
      .then((res) => {
        if (active) {
          setSpec(res);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (active) {
          setError(String(err));
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [author, name, version]);

  const renderType = (type: any): string => {
    if (!type) return "Unknown";
    if (typeof type === "string") return type;
    if (type && typeof type === "object") {
      if (type.PrimitiveScalar) return type.PrimitiveScalar;
      if (type.PrimitiveList) return `[${type.PrimitiveList}]`;
      if (type.PrimitiveFixedList) return `[${type.PrimitiveFixedList[0]}; ${type.PrimitiveFixedList[1]}]`;
      if (type.List) return `[${renderType(type.List[0])}]`;
      if (type.Map) return `Map<${renderType(type.Map.key)}, ${renderType(type.Map.value)}>`;
      if (type.Group) {
        return `{ ${type.Group.map((f: any) => `${f.name}: ${renderType(f.data_type)}`).join(", ")} }`;
      }
      return JSON.stringify(type);
    }
    return "Unknown";
  };

  const renderSchemaTable = (schema?: PyroSchema) => {
    if (!schema || !schema.fields || schema.fields.length === 0) {
      return <p className="text-muted small">Empty schema</p>;
    }

    return (
      <div className="table-container mt-10">
        <table className="table spec-table">
          <thead>
            <tr>
              <th>Field</th>
              <th>Type</th>
              <th>Nullable</th>
              <th>Documentation</th>
            </tr>
          </thead>
          <tbody>
            {schema.fields.map((field, idx) => (
              <tr key={idx}>
                <td className="font-semibold text-primary">{field.name}</td>
                <td>
                  <span className="code-text">{renderType(field.data_type)}</span>
                </td>
                <td>{field.nullable ? "Yes" : "No"}</td>
                <td className="text-muted text-sm">{field.documentation || "-"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  };

  // Helper variables with fallback defaults to prevent crashes
  const specHash = spec?.hash || "Unknown";
  const func = spec?.func || {};
  const funcName = func.name || "main";
  const funcDescription = func.description;
  const funcKind = func.kind;
  const inputSchema = func.input;
  const outputSchema = func.output;
  const capabilities = spec?.capabilities || [];
  const interconnect = spec?.interconnect || {};

  return (
    <div className="spec-view-container">
      <div className="spec-view-header">
        <div className="flex items-center gap-15">
          {onBack && (
            <button onClick={onBack} className="btn btn-secondary btn-sm btn-back">
              ← Back
            </button>
          )}
          <div>
            <span className="badge badge-online">Playbook Spec</span>
            <h2 className="spec-title mt-5">
              {author}/{name} <span className="text-muted text-md">v{version}</span>
            </h2>
          </div>
        </div>
      </div>

      {loading && (
        <div className="spec-loading">
          <div className="spinner"></div>
          <p className="mt-10 text-muted">Loading playbook specification...</p>
        </div>
      )}

      {error && (
        <div className="card border-danger bg-danger-glow p-20 mt-20">
          <h3 className="text-danger font-semibold">Failed to load specification</h3>
          <p className="mt-10 text-sm">{error}</p>
        </div>
      )}

      {!loading && !error && spec && (
        <div className="spec-content mt-20">
          {/* Metadata Card */}
          <div className="card p-20 mb-20">
            <h3 className="section-title">Playbook Metadata</h3>
            <div className="info-list mt-10">
              <div className="info-row">
                <span className="label">Spec Hash</span>
                <span className="value code-text">{specHash}</span>
              </div>
              {funcKind && (
                <div className="info-row">
                  <span className="label">Execution Model</span>
                  <span className="value badge badge-online">{String(funcKind).toUpperCase()}</span>
                </div>
              )}
            </div>
          </div>

          {/* Main Function spec */}
          <div className="card p-24 mb-20">
            <div className="class-header mb-15">
              <h4 className="class-name">Main Function: {funcName}</h4>
              {funcDescription && (
                <p className="class-desc text-muted mt-5">{funcDescription}</p>
              )}
            </div>

            <div className="schema-group mb-20">
              <h5 className="schema-title">Input Schema</h5>
              {renderSchemaTable(inputSchema)}
            </div>

            <div className="schema-group">
              <h5 className="schema-title">Output Schema</h5>
              {renderSchemaTable(outputSchema)}
            </div>
          </div>

          <div className="grid-layout mt-20">
            {/* Required Capabilities */}
            <div className="card p-20">
              <h3 className="section-title mb-15">Capabilities Required</h3>
              {capabilities.length === 0 ? (
                <p className="text-muted">No external capabilities required by this playbook.</p>
              ) : (
                <div className="capability-pills">
                  {capabilities.map((cap: any, idx: number) => {
                    const cAuthor = cap?.author || "unknown";
                    const cPackage = cap?.package || "unknown";
                    const cVersion = cap?.version || "";
                    return (
                      <span key={idx} className="cap-pill" style={{ padding: "6px 12px", fontSize: "12px" }}>
                        {cAuthor}/{cPackage} {cVersion && `v${cVersion}`}
                      </span>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Interconnect map */}
            <div className="card p-20">
              <h3 className="section-title mb-15">Sub-playbook Links (Interconnect)</h3>
              {Object.keys(interconnect).length === 0 ? (
                <p className="text-muted">No interconnected playbooks.</p>
              ) : (
                <div className="table-container">
                  <table className="table">
                    <thead>
                      <tr>
                        <th>Alias</th>
                        <th>Playbook</th>
                      </tr>
                    </thead>
                    <tbody>
                      {Object.entries(interconnect).map(([alias, target]: [string, any], idx) => {
                        const tAuthor = target?.author || "unknown";
                        const tPackage = target?.package || "unknown";
                        const tVersion = target?.version || "";
                        return (
                          <tr key={idx}>
                            <td className="font-semibold text-primary">{alias}</td>
                            <td className="text-muted">
                              {tAuthor}/{tPackage} {tVersion && `v${tVersion}`}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
