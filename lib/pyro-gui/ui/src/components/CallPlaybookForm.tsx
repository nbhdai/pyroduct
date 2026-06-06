import React, { useState, useEffect } from "react";
import { PlaybookSpec } from "../types";
import { LogViewer } from "./LogViewer";

export interface ExecutionDetails {
  row_index: number;
  input: any;
  success?: any;
  failure?: any;
  logs: any;
  is_success: boolean;
}

export const unpackExecutionRecord = (rec: any): ExecutionDetails | null => {
  if (!rec) return null;
  const inner = rec.Normal || rec.Session || rec.SessionDiff;
  if (!inner) return null;
  if (inner.Success) {
    return {
      row_index: inner.Success.row_index,
      input: inner.Success.input,
      success: inner.Success.success,
      logs: inner.Success.logs,
      is_success: true
    };
  } else if (inner.Failure) {
    return {
      row_index: inner.Failure.row_index,
      input: inner.Failure.input,
      failure: inner.Failure.failure,
      logs: inner.Failure.logs,
      is_success: false
    };
  }
  return null;
};

interface CallPlaybookFormProps {
  playbookName: string;
  playbookSpec?: PlaybookSpec;
  onSubmit: (name: string, payload: any) => Promise<any>;
  onSuccess?: () => void;
}

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

const isGroupType = (type: any): boolean => {
  return !!(type && typeof type === "object" && type.Group);
};

const isComplexType = (type: any): boolean => {
  if (!type) return false;
  if (typeof type === "string") return false;
  if (typeof type === "object") {
    if (type.PrimitiveScalar) return false;
    if (type.Group) return false; // Handled separately as individual fields
    return true;
  }
  return false;
};

const getDefaultValueForType = (type: any): any => {
  if (!type) return null;
  if (typeof type === "string") {
    switch (type) {
      case "Null": return null;
      case "Str": return "";
      case "Timestamp": return new Date().toISOString();
      default: return null;
    }
  }
  if (typeof type === "object") {
    if (type.PrimitiveScalar) {
      const scalar = type.PrimitiveScalar;
      if (scalar === "Bool") return false;
      return 0;
    }
    if (type.PrimitiveList) {
      return [];
    }
    if (type.PrimitiveFixedList) {
      const [elemType, size] = type.PrimitiveFixedList;
      const val = elemType === "Bool" ? false : 0;
      return Array(size).fill(val);
    }
    if (type.List) {
      return [];
    }
    if (type.Map) {
      return {};
    }
    if (type.Group) {
      const obj: Record<string, any> = {};
      const fields = type.Group || [];
      fields.forEach((field: any) => {
        if (field.nullable) {
          obj[field.name] = null;
        } else {
          obj[field.name] = getDefaultValueForType(field.data_type);
        }
      });
      return obj;
    }
  }
  return null;
};

const getInitialValueForType = (type: any): any => {
  if (isGroupType(type)) {
    const obj: Record<string, any> = {};
    const fields = type.Group || [];
    fields.forEach((field: any) => {
      if (field.nullable) {
        obj[field.name] = null;
      } else {
        obj[field.name] = getInitialValueForType(field.data_type);
      }
    });
    return obj;
  } else if (isComplexType(type)) {
    return JSON.stringify(getDefaultValueForType(type), null, 2);
  } else {
    return getDefaultValueForType(type);
  }
};

const getValueAtPath = (obj: any, path: string[]): any => {
  let current = obj;
  for (const key of path) {
    if (current === undefined || current === null) return undefined;
    current = current[key];
  }
  return current;
};

const setValueAtPath = (obj: any, path: string[], value: any): any => {
  const newObj = { ...obj };
  let current = newObj;
  for (let i = 0; i < path.length - 1; i++) {
    const key = path[i];
    current[key] = { ...current[key] };
    current = current[key];
  }
  current[path[path.length - 1]] = value;
  return newObj;
};

const buildPayloadForType = (val: any, type: any, fieldName: string, nullable: boolean): any => {
  if (isGroupType(type)) {
    const obj: Record<string, any> = {};
    const subFields = type.Group || [];
    for (const subField of subFields) {
      const subVal = val ? val[subField.name] : undefined;
      obj[subField.name] = buildPayloadForType(subVal, subField.data_type, `${fieldName}.${subField.name}`, subField.nullable);
    }
    return obj;
  }

  if (isComplexType(type)) {
    if (val === undefined || val === null || val === "") {
      return null;
    }
    try {
      return JSON.parse(val);
    } catch (err: any) {
      return null;
    }
  }

  // Primitive types
  if (typeof type === "string") {
    if (type === "Null") {
      return null;
    } else {
      return val !== undefined && val !== null ? String(val) : "";
    }
  }

  if (type && typeof type === "object" && type.PrimitiveScalar) {
    const scalar = type.PrimitiveScalar;
    if (scalar === "Bool") {
      return Boolean(val);
    } else {
      if (val === "" || val === undefined || val === null) {
        return null;
      }
      const num = Number(val);
      return isNaN(num) ? 0 : num;
    }
  }

  return val;
};

function PrettyJson({ data }: { data: any }) {
  if (data === undefined || data === null) {
    return <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>null</span>;
  }
  return (
    <pre style={{
      margin: 0,
      padding: "12px",
      borderRadius: "6px",
      backgroundColor: "#0d0e12",
      border: "1px solid var(--bg-card-border)",
      color: "#e3e3e6",
      fontFamily: "monospace",
      fontSize: "13px",
      overflowX: "auto",
      whiteSpace: "pre-wrap",
      wordBreak: "break-all"
    }}>
      {JSON.stringify(data, null, 2)}
    </pre>
  );
}

interface NormalExecutionViewerProps {
  record: any;
}

function NormalExecutionViewer({ record }: NormalExecutionViewerProps) {
  const isSuccess = !!record.Success;
  const data = record.Success || record.Failure;
  if (!data) return null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      {/* Header Status Badge */}
      <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
        <span style={{
          display: "inline-flex",
          alignItems: "center",
          padding: "6px 12px",
          borderRadius: "20px",
          fontSize: "12px",
          fontWeight: 700,
          letterSpacing: "0.05em",
          textTransform: "uppercase",
          backgroundColor: isSuccess ? "rgba(16, 185, 129, 0.15)" : "rgba(239, 68, 68, 0.15)",
          color: isSuccess ? "#10b981" : "#ef4444",
          border: `1px solid ${isSuccess ? "rgba(16, 185, 129, 0.3)" : "rgba(239, 68, 68, 0.3)"}`
        }}>
          {isSuccess ? "● Success" : "▲ Failure"}
        </span>
        <span style={{ fontSize: "13px", color: "var(--text-muted)" }}>
          Row Index: <strong>{data.row_index}</strong>
        </span>
      </div>

      {/* Input / Output Panels */}
      <div style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
        <div>
          <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>Input Payload</h5>
          <PrettyJson data={data.input} />
        </div>
        
        {isSuccess ? (
          <div>
            <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>Success Result</h5>
            <PrettyJson data={data.success} />
          </div>
        ) : (
          <div>
            <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "#ef4444" }}>Execution Error</h5>
            <div style={{
              padding: "12px",
              borderRadius: "6px",
              backgroundColor: "rgba(239, 68, 68, 0.05)",
              border: "1px solid rgba(239, 68, 68, 0.2)",
              color: "#f87171",
              fontFamily: "monospace",
              fontSize: "13px"
            }}>
              {typeof data.failure === "string" ? data.failure : (data.failure?.Ok?.message || JSON.stringify(data.failure) || "Unknown module error")}
            </div>
          </div>
        )}
      </div>

      {/* Logs on Failure ONLY */}
      {!isSuccess && (
        <div style={{ borderTop: "1px solid var(--bg-card-border)", paddingTop: "15px", marginTop: "10px" }}>
          <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-muted)" }}>Execution Failure Logs</h4>
          <LogViewer logs={data.logs} />
        </div>
      )}
    </div>
  );
}

interface SessionExecutionViewerProps {
  record: any;
}

function SessionExecutionViewer({ record }: SessionExecutionViewerProps) {
  const isSuccess = !!record.Success;
  const data = record.Success || record.Failure;
  if (!data) return null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      {/* Header Status Badge */}
      <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
        <span style={{
          display: "inline-flex",
          alignItems: "center",
          padding: "6px 12px",
          borderRadius: "20px",
          fontSize: "12px",
          fontWeight: 700,
          letterSpacing: "0.05em",
          textTransform: "uppercase",
          backgroundColor: isSuccess ? "rgba(16, 185, 129, 0.15)" : "rgba(239, 68, 68, 0.15)",
          color: isSuccess ? "#10b981" : "#ef4444",
          border: `1px solid ${isSuccess ? "rgba(16, 185, 129, 0.3)" : "rgba(239, 68, 68, 0.3)"}`
        }}>
          {isSuccess ? "● Session Success" : "▲ Session Failure"}
        </span>
        <span style={{ fontSize: "13px", color: "var(--text-muted)" }}>
          Session ID: <strong>{data.row_index}</strong>
        </span>
      </div>

      {/* Prior Steps History / Timeline */}
      {data.prior && data.prior.length > 0 && (
        <div style={{
          backgroundColor: "rgba(255, 255, 255, 0.01)",
          border: "1px solid var(--bg-card-border)",
          borderRadius: "var(--border-radius)",
          padding: "16px",
        }}>
          <h5 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-muted)" }}>
            Session Step History ({data.prior.length} prior steps)
          </h5>
          <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
            {data.prior.map((priorStepInput: any, index: number) => (
              <div key={index} style={{
                display: "flex",
                flexDirection: "column",
                padding: "10px 14px",
                backgroundColor: "rgba(0, 0, 0, 0.15)",
                borderLeft: "3px solid var(--color-primary)",
                borderRadius: "4px"
              }}>
                <span style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", marginBottom: "4px" }}>
                  Step #{index + 1} Input
                </span>
                <pre style={{ margin: 0, fontSize: "12px", fontFamily: "monospace", color: "#b9bbbe", overflowX: "auto" }}>
                  {JSON.stringify(priorStepInput)}
                </pre>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Current Step Section */}
      <div style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
        <div>
          <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>
            Current Step Input (Step #{ (data.prior?.length || 0) + 1 })
          </h5>
          <PrettyJson data={data.input} />
        </div>
        
        {isSuccess ? (
          <div>
            <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>Current Step Output</h5>
            <PrettyJson data={data.success} />
          </div>
        ) : (
          <div>
            <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "#ef4444" }}>Execution Error</h5>
            <div style={{
              padding: "12px",
              borderRadius: "6px",
              backgroundColor: "rgba(239, 68, 68, 0.05)",
              border: "1px solid rgba(239, 68, 68, 0.2)",
              color: "#f87171",
              fontFamily: "monospace",
              fontSize: "13px"
            }}>
              {typeof data.failure === "string" ? data.failure : (data.failure?.Ok?.message || JSON.stringify(data.failure) || "Unknown session error")}
            </div>
          </div>
        )}
      </div>

      {/* Logs on Failure ONLY */}
      {!isSuccess && (
        <div style={{ borderTop: "1px solid var(--bg-card-border)", paddingTop: "15px", marginTop: "10px" }}>
          <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-muted)" }}>Execution Failure Logs</h4>
          <LogViewer logs={data.logs} />
        </div>
      )}
    </div>
  );
}

interface SessionDiffExecutionViewerProps {
  record: any;
}

function SessionDiffExecutionViewer({ record }: SessionDiffExecutionViewerProps) {
  const isSuccess = !!record.Success;
  const data = record.Success || record.Failure;
  if (!data) return null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      {/* Header Status Badge */}
      <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
        <span style={{
          display: "inline-flex",
          alignItems: "center",
          padding: "6px 12px",
          borderRadius: "20px",
          fontSize: "12px",
          fontWeight: 700,
          letterSpacing: "0.05em",
          textTransform: "uppercase",
          backgroundColor: isSuccess ? "rgba(16, 185, 129, 0.15)" : "rgba(239, 68, 68, 0.15)",
          color: isSuccess ? "#10b981" : "#ef4444",
          border: `1px solid ${isSuccess ? "rgba(16, 185, 129, 0.3)" : "rgba(239, 68, 68, 0.3)"}`
        }}>
          {isSuccess ? "● Session Diff Success" : "▲ Session Diff Failure"}
        </span>
        <span style={{ fontSize: "13px", color: "var(--text-muted)" }}>
          Session ID: <strong>{data.row_index}</strong>
        </span>
      </div>

      {/* Prior Steps History / Timeline */}
      {data.prior_input && data.prior_input.length > 0 && (
        <div style={{
          backgroundColor: "rgba(255, 255, 255, 0.01)",
          border: "1px solid var(--bg-card-border)",
          borderRadius: "var(--border-radius)",
          padding: "16px",
        }}>
          <h5 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-muted)" }}>
            Session Diff Step History ({data.prior_input.length} prior steps)
          </h5>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {data.prior_input.map((priorIn: any, index: number) => {
              const priorOut = data.prior_output?.[index];
              return (
                <div key={index} style={{
                  display: "flex",
                  flexDirection: "column",
                  padding: "12px 14px",
                  backgroundColor: "rgba(0, 0, 0, 0.15)",
                  borderLeft: "3px solid #8b5cf6",
                  borderRadius: "4px",
                  gap: "6px"
                }}>
                  <div style={{ fontSize: "11px", fontWeight: 700, color: "var(--text-muted)" }}>
                    Step #{index + 1}
                  </div>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "15px" }}>
                    <div>
                      <span style={{ fontSize: "10px", display: "block", color: "var(--text-muted)", marginBottom: "2px" }}>Input</span>
                      <pre style={{ margin: 0, fontSize: "11px", fontFamily: "monospace", color: "#b9bbbe", overflowX: "auto" }}>
                        {JSON.stringify(priorIn)}
                      </pre>
                    </div>
                    <div>
                      <span style={{ fontSize: "10px", display: "block", color: "var(--text-muted)", marginBottom: "2px" }}>Output</span>
                      <pre style={{ margin: 0, fontSize: "11px", fontFamily: "monospace", color: "#b9bbbe", overflowX: "auto" }}>
                        {JSON.stringify(priorOut || null)}
                      </pre>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Current Step Section */}
      <div style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
        <div>
          <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>
            Current Step Input (Step #{ (data.prior_input?.length || 0) + 1 })
          </h5>
          <PrettyJson data={data.input} />
        </div>
        
        {isSuccess ? (
          <div>
            <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>Current Step Output</h5>
            <PrettyJson data={data.success} />
          </div>
        ) : (
          <div>
            <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "#ef4444" }}>Execution Error</h5>
            <div style={{
              padding: "12px",
              borderRadius: "6px",
              backgroundColor: "rgba(239, 68, 68, 0.05)",
              border: "1px solid rgba(239, 68, 68, 0.2)",
              color: "#f87171",
              fontFamily: "monospace",
              fontSize: "13px"
            }}>
              {typeof data.failure === "string" ? data.failure : (data.failure?.Ok?.message || JSON.stringify(data.failure) || "Unknown session-diff error")}
            </div>
          </div>
        )}
      </div>

      {/* Logs on Failure ONLY */}
      {!isSuccess && (
        <div style={{ borderTop: "1px solid var(--bg-card-border)", paddingTop: "15px", marginTop: "10px" }}>
          <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-muted)" }}>Execution Failure Logs</h4>
          <LogViewer logs={data.logs} />
        </div>
      )}
    </div>
  );
}

interface ServerExecutionRecordViewerProps {
  record: any;
}

function ServerExecutionRecordViewer({ record }: ServerExecutionRecordViewerProps) {
  if (!record) return null;

  if (record.Normal !== undefined) {
    return <NormalExecutionViewer record={record.Normal} />;
  }
  if (record.Session !== undefined) {
    return <SessionExecutionViewer record={record.Session} />;
  }
  if (record.SessionDiff !== undefined) {
    return <SessionDiffExecutionViewer record={record.SessionDiff} />;
  }

  return (
    <div>
      <h5 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "6px", color: "var(--text-muted)" }}>Result Payload</h5>
      <PrettyJson data={record} />
    </div>
  );
}

export function CallPlaybookForm({ playbookName, playbookSpec, onSubmit, onSuccess }: CallPlaybookFormProps) {
  const [formValues, setFormValues] = useState<Record<string, any>>({});
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [executionRecord, setExecutionRecord] = useState<any | null>(null);

  const handlePathValueChange = (path: string[], value: any) => {
    setFormValues((prev) => setValueAtPath(prev, path, value));
  };

  // Reset form when target changes
  useEffect(() => {
    setResult(null);
    setExecutionRecord(null);
    setSubmitting(false);

    if (playbookSpec && playbookSpec.func && playbookSpec.func.input) {
      const fields = playbookSpec.func.input.fields || [];
      const initialValues: Record<string, any> = {};
      fields.forEach((field: any) => {
        if (field.nullable) {
          initialValues[field.name] = null;
        } else {
          initialValues[field.name] = getInitialValueForType(field.data_type);
        }
      });
      setFormValues(initialValues);
    } else {
      setFormValues({});
    }
  }, [playbookName, playbookSpec]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setResult(null);
    setExecutionRecord(null);
    
    const fields = playbookSpec?.func?.input?.fields || [];
    const payload: Record<string, any> = {};

    try {
      for (const field of fields) {
        const val = formValues[field.name];
        payload[field.name] = buildPayloadForType(val, field.data_type, field.name, field.nullable);
      }
    } catch (err: any) {
      const errMsg = err.message;
      setResult(`Error during payload construction: ${errMsg}`);
      return;
    }

    setSubmitting(true);
    try {
      console.log("CallPlaybookForm: calling onSubmit with payload:", payload);
      const res = await onSubmit(playbookName, payload);
      console.log("CallPlaybookForm: onSubmit resolved, result:", res);
      
      setExecutionRecord(res);
      if (res) {
        const inner = res.Normal || res.Session || res.SessionDiff;
        if (inner && inner.Success && onSuccess) {
          onSuccess();
        }
      }
    } catch (err: any) {
      setResult(`Error: ${err}`);
    } finally {
      setSubmitting(false);
    }
  };

  const renderField = (field: any, path: string[]) => {
    const isGroup = isGroupType(field.data_type);
    const isComplex = !isGroup && isComplexType(field.data_type);
    const isBool = !isGroup && !isComplex && typeof field.data_type === "object" && field.data_type.PrimitiveScalar && field.data_type.PrimitiveScalar === "Bool";
    const isNum = !isGroup && !isComplex && typeof field.data_type === "object" && field.data_type.PrimitiveScalar;
    const val = getValueAtPath(formValues, path);

    const labelText = path.join(".");

    if (isGroup) {
      const subFields = field.data_type.Group || [];
      return (
        <div className="form-group-box" key={labelText} style={{
          border: "1px solid var(--bg-card-border)",
          borderRadius: "var(--border-radius)",
          padding: "20px",
          marginBottom: "20px",
          backgroundColor: "rgba(255, 255, 255, 0.01)"
        }}>
          <label style={{ display: "block", marginBottom: "10px", fontWeight: 600, fontSize: "14px" }}>
            {field.name}{" "}
            <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
              (Struct){field.nullable ? " - Optional" : " *"}
            </span>
          </label>
          {field.documentation && (
            <p className="field-desc" style={{ margin: "2px 0 10px 0", fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic" }}>
              {field.documentation}
            </p>
          )}
          <div style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
            {subFields.map((subField: any) => renderField(subField, [...path, subField.name]))}
          </div>
        </div>
      );
    }

    return (
      <div className="form-group" key={labelText} style={{ marginBottom: "15px" }}>
        {!isBool && (
          <label htmlFor={`field-${labelText}`} style={{ display: "block", marginBottom: "6px", fontWeight: 600 }}>
            {field.name}{" "}
            <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
              ({renderType(field.data_type)}){field.nullable ? " - Optional" : " *"}
            </span>
          </label>
        )}

        {field.documentation && (
          <p className="field-desc" style={{ margin: "2px 0 6px 0", fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic" }}>
            {field.documentation}
          </p>
        )}

        {isComplex ? (
          <textarea
            id={`field-${labelText}`}
            rows={4}
            value={val !== undefined && val !== null ? val : ""}
            onChange={(e) => handlePathValueChange(path, e.target.value)}
            placeholder={`Enter JSON for ${renderType(field.data_type)}`}
          />
        ) : isBool ? (
          <div className="checkbox-wrapper" style={{ display: "flex", alignItems: "center", gap: "8px", padding: "4px 0" }}>
            <input
              type="checkbox"
              id={`field-${labelText}`}
              checked={!!val}
              onChange={(e) => handlePathValueChange(path, e.target.checked)}
              style={{ width: "18px", height: "18px", accentColor: "var(--color-primary)", cursor: "pointer" }}
            />
            <label htmlFor={`field-${labelText}`} style={{ margin: 0, cursor: "pointer", userSelect: "none", fontWeight: 600 }}>
              {field.name}{" "}
              <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
                (Bool){field.nullable ? " - Optional" : " *"}
              </span>
            </label>
          </div>
        ) : isNum ? (
          <input
            type="number"
            id={`field-${labelText}`}
            value={val !== undefined && val !== null ? val : ""}
            onChange={(e) => handlePathValueChange(path, e.target.value)}
            placeholder={field.nullable ? "Optional" : "Required number"}
          />
        ) : (
          <input
            type="text"
            id={`field-${labelText}`}
            value={val !== undefined && val !== null ? val : ""}
            onChange={(e) => handlePathValueChange(path, e.target.value)}
            placeholder={field.nullable ? "Optional" : "Required string"}
          />
        )}
      </div>
    );
  };

  return (
    <div className="card" style={{ marginBottom: "20px" }}>
      <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "15px" }}>
        Execute Call
      </h3>
      <form onSubmit={handleSubmit} noValidate>
        {playbookSpec && playbookSpec.func && playbookSpec.func.input && playbookSpec.func.input.fields && playbookSpec.func.input.fields.length > 0 ? (
          <div className="form-fields-container" style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
            {playbookSpec.func.input.fields.map((field) => renderField(field, [field.name]))}
          </div>
        ) : (
          <div className="empty-state" style={{ padding: "20px" }}>
            <span className="empty-icon">⚙</span>
            <p>No input fields required or spec not loaded.</p>
          </div>
        )}

        <div className="form-actions" style={{ marginTop: "20px" }}>
          <button type="submit" disabled={submitting} className="btn btn-primary" style={{ padding: "8px 20px" }}>
            {submitting ? "Calling..." : "Send Call"}
          </button>
        </div>
      </form>

      {result !== null && (
        <div className="mt-20">
          <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "8px" }}>Result Output</h4>
          <pre className="console-box" style={{ marginTop: "4px", maxHeight: "200px", overflowY: "auto" }}>
            {result}
          </pre>
        </div>
      )}

      {executionRecord && (
        <div className="mt-20" style={{ borderTop: "1px solid var(--bg-card-border)", paddingTop: "15px" }}>
          <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-muted)" }}>Execution Result Details</h4>
          <ServerExecutionRecordViewer record={executionRecord} />
        </div>
      )}
    </div>
  );
}
