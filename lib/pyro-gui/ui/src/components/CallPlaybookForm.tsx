import React, { useState, useEffect } from "react";
import { PlaybookSpec } from "../types";

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

const isComplexType = (type: any): boolean => {
  if (!type) return false;
  if (typeof type === "string") return false;
  if (typeof type === "object") {
    if (type.PrimitiveScalar) return false;
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

export function CallPlaybookForm({ playbookName, playbookSpec, onSubmit, onSuccess }: CallPlaybookFormProps) {
  const [formValues, setFormValues] = useState<Record<string, any>>({});
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  const handleValueChange = (name: string, value: any) => {
    setFormValues((prev) => ({
      ...prev,
      [name]: value,
    }));
  };

  // Reset form when target changes
  useEffect(() => {
    setResult(null);
    setSubmitting(false);

    if (playbookSpec && playbookSpec.func && playbookSpec.func.input) {
      const fields = playbookSpec.func.input.fields || [];
      const initialValues: Record<string, any> = {};
      fields.forEach((field: any) => {
        if (field.nullable) {
          initialValues[field.name] = null;
        } else {
          const defVal = getDefaultValueForType(field.data_type);
          if (isComplexType(field.data_type)) {
            initialValues[field.name] = JSON.stringify(defVal, null, 2);
          } else {
            initialValues[field.name] = defVal;
          }
        }
      });
      setFormValues(initialValues);
    } else {
      setFormValues({});
    }
  }, [playbookName, playbookSpec]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    const fields = playbookSpec?.func?.input?.fields || [];
    const payload: Record<string, any> = {};

    for (const field of fields) {
      const val = formValues[field.name];

      if (val === undefined || val === null) {
        if (field.nullable) {
          payload[field.name] = null;
          continue;
        } else {
          alert(`Field "${field.name}" is required.`);
          return;
        }
      }

      if (isComplexType(field.data_type)) {
        try {
          payload[field.name] = JSON.parse(val);
        } catch (err: any) {
          alert(`Invalid JSON in field "${field.name}": ` + err.message);
          return;
        }
      } else {
        const type = field.data_type;
        if (typeof type === "string") {
          if (type === "Null") {
            payload[field.name] = null;
          } else {
            if (val === "" && field.nullable) {
              payload[field.name] = null;
            } else {
              payload[field.name] = String(val);
            }
          }
        } else if (type && typeof type === "object" && type.PrimitiveScalar) {
          const scalar = type.PrimitiveScalar;
          if (scalar === "Bool") {
            payload[field.name] = Boolean(val);
          } else {
            if (val === "" && field.nullable) {
              payload[field.name] = null;
            } else {
              const num = Number(val);
              if (isNaN(num)) {
                alert(`Field "${field.name}" must be a number.`);
                return;
              }
              payload[field.name] = num;
            }
          }
        } else {
          payload[field.name] = val;
        }
      }
    }

    setSubmitting(true);
    setResult(null);
    try {
      const res = await onSubmit(playbookName, payload);
      setResult(JSON.stringify(res, null, 2));
      if (onSuccess) {
        onSuccess();
      }
    } catch (err: any) {
      setResult(`Error: ${err}`);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="card" style={{ marginBottom: "20px" }}>
      <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "15px" }}>
        Execute Call
      </h3>
      <form onSubmit={handleSubmit}>
        {playbookSpec && playbookSpec.func && playbookSpec.func.input && playbookSpec.func.input.fields && playbookSpec.func.input.fields.length > 0 ? (
          <div className="form-fields-container" style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
            {playbookSpec.func.input.fields.map((field) => {
              const isComplex = isComplexType(field.data_type);
              const isBool = !isComplex && typeof field.data_type === "object" && field.data_type.PrimitiveScalar && field.data_type.PrimitiveScalar === "Bool";
              const isNum = !isComplex && typeof field.data_type === "object" && field.data_type.PrimitiveScalar;

              return (
                <div className="form-group" key={field.name} style={{ marginBottom: "15px" }}>
                  {!isBool && (
                    <label htmlFor={`field-${field.name}`} style={{ display: "block", marginBottom: "6px", fontWeight: 600 }}>
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
                      id={`field-${field.name}`}
                      rows={4}
                      value={formValues[field.name] !== undefined && formValues[field.name] !== null ? formValues[field.name] : ""}
                      onChange={(e) => handleValueChange(field.name, e.target.value)}
                      required={!field.nullable}
                      placeholder={`Enter JSON for ${renderType(field.data_type)}`}
                    />
                  ) : isBool ? (
                    <div className="checkbox-wrapper" style={{ display: "flex", alignItems: "center", gap: "8px", padding: "4px 0" }}>
                      <input
                        type="checkbox"
                        id={`field-${field.name}`}
                        checked={!!formValues[field.name]}
                        onChange={(e) => handleValueChange(field.name, e.target.checked)}
                        style={{ width: "18px", height: "18px", accentColor: "var(--color-primary)", cursor: "pointer" }}
                      />
                      <label htmlFor={`field-${field.name}`} style={{ margin: 0, cursor: "pointer", userSelect: "none", fontWeight: 600 }}>
                        {field.name}{" "}
                        <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
                          (Bool){field.nullable ? " - Optional" : " *"}
                        </span>
                      </label>
                    </div>
                  ) : isNum ? (
                    <input
                      type="number"
                      id={`field-${field.name}`}
                      value={formValues[field.name] !== undefined && formValues[field.name] !== null ? formValues[field.name] : ""}
                      onChange={(e) => handleValueChange(field.name, e.target.value)}
                      required={!field.nullable}
                      placeholder={field.nullable ? "Optional" : "Required number"}
                    />
                  ) : (
                    <input
                      type="text"
                      id={`field-${field.name}`}
                      value={formValues[field.name] !== undefined && formValues[field.name] !== null ? formValues[field.name] : ""}
                      onChange={(e) => handleValueChange(field.name, e.target.value)}
                      required={!field.nullable}
                      placeholder={field.nullable ? "Optional" : "Required string"}
                    />
                  )}
                </div>
              );
            })}
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
          <h4 style={{ fontSize: "14px", fontWeight: 600 }}>Result</h4>
          <pre className="console-box" style={{ marginTop: "8px", maxHeight: "250px", overflowY: "auto" }}>
            {result}
          </pre>
        </div>
      )}
    </div>
  );
}
