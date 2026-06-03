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
  const [sourceCode, setSourceCode] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewTab, setViewTab] = useState<"metadata" | "source">("metadata");

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    setSourceCode(null);

    Promise.all([
      invoke("get_playbook_spec", { author, name, version }),
      invoke("get_playbook_source", { author, name, version })
    ])
      .then(([specRes, srcRes]) => {
        if (active) {
          setSpec(specRes);
          setSourceCode(srcRes as string);
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
        <div className="tabs-sub" style={{ marginBottom: "20px" }}>
          <button
            className={`sub-tab-btn ${viewTab === "metadata" ? "active" : ""}`}
            onClick={() => setViewTab("metadata")}
          >
            Metadata
          </button>
          {sourceCode && (
            <button
              className={`sub-tab-btn ${viewTab === "source" ? "active" : ""}`}
              onClick={() => setViewTab("source")}
            >
              Source Code
            </button>
          )}
        </div>
      )}

      {!loading && !error && spec && (
        <div className="spec-content">
          {viewTab === "metadata" && (
            <>
              {/* Playbook Description at the top */}
              {funcDescription && (
                <div className="card p-20 mb-20" style={{ borderLeft: "4px solid var(--color-primary)" }}>
                  <p className="description-text" style={{ fontSize: "15px", color: "var(--text-main)", fontStyle: "italic" }}>
                    "{funcDescription}"
                  </p>
                </div>
              )}

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
            </>
          )}

          {viewTab === "source" && sourceCode && (
            <div className="card p-24">
              <div className="code-container" style={{ marginTop: "10px" }}>
                <pre className="code-block">
                  <code dangerouslySetInnerHTML={{ __html: highlightRust(sourceCode) }} />
                </pre>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function highlightRust(code: string) {
  if (!code) return "";

  const escapeHtml = (unsafe: string) => {
    return unsafe
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  };

  const escaped = escapeHtml(code);
  let highlighted = escaped;

  // 1. Comments: // ... or /* ... */
  const comments: string[] = [];
  highlighted = highlighted.replace(/(\/\/.*|\/\*[\s\S]*?\*\/)/g, (match) => {
    comments.push(match);
    return `__COMMENT_PLACEHOLDER_${comments.length - 1}__`;
  });

  // 2. Strings: "..."
  const strings: string[] = [];
  highlighted = highlighted.replace(/(&quot;[\s\S]*?&quot;)/g, (match) => {
    strings.push(match);
    return `__STRING_PLACEHOLDER_${strings.length - 1}__`;
  });

  // 3. Attributes / Macros: #\[...\] or #!\[...\]
  const attributes: string[] = [];
  highlighted = highlighted.replace(/(#!?\[[\s\S]*?\])/g, (match) => {
    attributes.push(match);
    return `__ATTR_PLACEHOLDER_${attributes.length - 1}__`;
  });

  // 4. Keywords
  const keywords = [
    "fn", "pub", "struct", "impl", "use", "let", "mut", "match", "if", "else",
    "return", "async", "await", "crate", "mod", "as", "for", "in", "loop",
    "while", "break", "continue", "unsafe", "type", "enum", "trait", "where", "const"
  ];
  const keywordRegex = new RegExp(`\\b(${keywords.join("|")})\\b`, "g");
  highlighted = highlighted.replace(keywordRegex, '<span class="hl-keyword">$1</span>');

  // 5. Common Types / Constants
  const types = ["String", "Result", "Option", "Value", "Vec", "Self", "self", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "bool", "Ok", "Err", "None", "Some"];
  const typesRegex = new RegExp(`\\b(${types.join("|")})\\b`, "g");
  highlighted = highlighted.replace(typesRegex, '<span class="hl-type">$1</span>');

  // 6. Restore placeholders
  // Restore Attributes
  attributes.forEach((val, idx) => {
    highlighted = highlighted.replace(`__ATTR_PLACEHOLDER_${idx}__`, `<span class="hl-attr">${val}</span>`);
  });

  // Restore Strings
  strings.forEach((val, idx) => {
    highlighted = highlighted.replace(`__STRING_PLACEHOLDER_${idx}__`, `<span class="hl-string">${val}</span>`);
  });

  // Restore Comments
  comments.forEach((val, idx) => {
    highlighted = highlighted.replace(`__COMMENT_PLACEHOLDER_${idx}__`, `<span class="hl-comment">${val}</span>`);
  });

  return highlighted;
}
