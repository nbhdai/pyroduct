import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { InterfaceSpec, PyroSchema } from "../types";

interface CapabilitySpecViewProps {
  author: string;
  name: string;
  version: string;
  onBack?: () => void;
}

export function CapabilitySpecView({ author, name, version, onBack }: CapabilitySpecViewProps) {
  const [spec, setSpec] = useState<InterfaceSpec | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);

    invoke("get_capability_interface_spec", { author, name, version })
      .then((res) => {
        if (active) {
          setSpec(res as InterfaceSpec);
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

  const renderSchemaTable = (schema: PyroSchema) => {
    if (!schema.fields || schema.fields.length === 0) {
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
                <td>{field.nullable ? "✅" : "❌"}</td>
                <td className="text-muted text-sm">{field.documentation || "-"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  };

  return (
    <div className="spec-view-container">
      <div className="spec-view-header">
        <div className="flex items-center gap-15">
          {onBack && (
            <button onClick={onBack} className="btn btn-secondary btn-sm btn-back">
              ⬅️ Back
            </button>
          )}
          <div>
            <span className="badge badge-online">Capability Spec</span>
            <h2 className="spec-title mt-5">
              {author}/{name} <span className="text-muted text-md">v{version}</span>
            </h2>
          </div>
        </div>
      </div>

      {loading && (
        <div className="spec-loading">
          <div className="spinner"></div>
          <p className="mt-10 text-muted">Loading interface specification...</p>
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
          {spec.description && (
            <div className="card p-20 mb-20">
              <p className="description-text">{spec.description}</p>
            </div>
          )}

          <div className="classes-section">
            <h3 className="section-title">Classes Defined</h3>
            {spec.classes.length === 0 ? (
              <p className="text-muted">No classes defined in this capability.</p>
            ) : (
              spec.classes.map((cls, clsIdx) => (
                <div key={clsIdx} className="card class-card p-24 mb-20">
                  <div className="class-header mb-15">
                    <h4 className="class-name">{cls.name}</h4>
                    {cls.description && <p className="class-desc text-muted mt-5">{cls.description}</p>}
                  </div>

                  {cls.config && cls.config.fields && cls.config.fields.length > 0 && (
                    <div className="schema-group mb-20">
                      <h5 className="schema-title">⚙️ Configuration Schema</h5>
                      {cls.config.documentation && (
                        <p className="text-muted text-sm mb-5">{cls.config.documentation}</p>
                      )}
                      {renderSchemaTable(cls.config)}
                    </div>
                  )}

                  {cls.client && cls.client.fields && cls.client.fields.length > 0 && (
                    <div className="schema-group mb-20">
                      <h5 className="schema-title">🔌 Client Schema</h5>
                      {cls.client.documentation && (
                        <p className="text-muted text-sm mb-5">{cls.client.documentation}</p>
                      )}
                      {renderSchemaTable(cls.client)}
                    </div>
                  )}

                  <div className="methods-group">
                    <h5 className="schema-title mb-10">⚡ Methods</h5>
                    {cls.methods.length === 0 ? (
                      <p className="text-muted text-sm">No methods defined</p>
                    ) : (
                      <div className="methods-list">
                        {cls.methods.map((method, mIdx) => (
                          <div key={mIdx} className="method-item p-16 mb-12">
                            <div className="method-header flex justify-between items-start">
                              <div>
                                <span className="method-name font-semibold text-primary">{method.name}</span>
                                <span className="text-muted text-sm ml-10">
                                  → returns <code className="code-text">{renderType(method.output)}</code>
                                </span>
                              </div>
                            </div>
                            {method.description && (
                              <p className="method-desc text-muted text-sm mt-5">{method.description}</p>
                            )}

                            {method.input && method.input.fields && method.input.fields.length > 0 && (
                              <div className="mt-10">
                                <span className="text-xs font-semibold text-muted">INPUT ARGUMENTS:</span>
                                {renderSchemaTable(method.input)}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}
