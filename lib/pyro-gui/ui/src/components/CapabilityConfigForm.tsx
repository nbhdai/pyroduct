import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { InterfaceSpec } from "../types";
import {
  renderType,
  isGroupType,
  isComplexType,
  getInitialValueForType,
  buildPayloadForType,
} from "../utils/configFormUtils";

/** Mirrors the Rust ConfiguredCapability struct */
export interface ConfiguredCapability {
  author: string;
  package: string;
  version: string;
  configuration: {
    classes: Record<string, any>;
  };
}

interface CapabilityConfigFormProps {
  /** The selected playbook's identity (author/package/version) */
  playbookIdent: { author: string; package: string; version: string };
  /** The selected playbook's capabilities list from its spec */
  capabilities: { author: string; package: string; version: string }[];
  /** Called whenever the user changes config values. Parent uses this on submit. */
  onChange: (configurations: ConfiguredCapability[]) => void;
}

interface CapabilityData {
  ident: { author: string; package: string; version: string };
  interfaceSpec: InterfaceSpec | null;
  /** Default class configs from the binary's ConfiguredCapability.configuration.classes */
  defaults: Record<string, any>;
  loading: boolean;
  error: string | null;
}

/**
 * Renders per-class config forms for each capability in a playbook.
 * Fetches InterfaceSpecs and pre-populates with binary defaults.
 */
export function CapabilityConfigForm({ playbookIdent, capabilities, onChange }: CapabilityConfigFormProps) {
  const [capData, setCapData] = useState<CapabilityData[]>([]);
  // formValues[capPackage][className] = { fieldName: value, ... }
  const [formValues, setFormValues] = useState<Record<string, Record<string, any>>>({});

  // Fetch interface specs and binary default configs for each capability
  useEffect(() => {
    if (!capabilities || capabilities.length === 0) {
      setCapData([]);
      setFormValues({});
      return;
    }

    let active = true;

    const fetchAll = async () => {
      // Fetch binary defaults for this playbook
      let binaryDefaults: ConfiguredCapability[] = [];
      try {
        binaryDefaults = (await invoke("get_playbook_configurations", {
          author: playbookIdent.author,
          name: playbookIdent.package,
          version: playbookIdent.version,
        })) as ConfiguredCapability[];
      } catch (e) {
        console.warn("Failed to fetch binary configurations:", e);
      }

      // Index defaults by package name for quick lookup
      const defaultsByPackage: Record<string, Record<string, any>> = {};
      for (const cfg of binaryDefaults) {
        if (cfg.configuration && cfg.configuration.classes) {
          defaultsByPackage[cfg.package] = cfg.configuration.classes;
        }
      }

      const results: CapabilityData[] = [];

      for (const cap of capabilities) {
        let interfaceSpec: InterfaceSpec | null = null;
        let error: string | null = null;

        try {
          interfaceSpec = (await invoke("get_capability_interface_spec", {
            author: cap.author,
            name: cap.package,
            version: cap.version,
          })) as InterfaceSpec;
        } catch (e) {
          error = String(e);
        }

        results.push({
          ident: cap,
          interfaceSpec,
          defaults: defaultsByPackage[cap.package] || {},
          loading: false,
          error,
        });
      }

      if (!active) return;
      setCapData(results);
    };

    // Set loading state
    setCapData(
      capabilities.map((cap) => ({
        ident: cap,
        interfaceSpec: null,
        defaults: {},
        loading: true,
        error: null,
      }))
    );

    fetchAll();
    return () => { active = false; };
  }, [capabilities, playbookIdent]);

  // Initialize form values from interface spec defaults when capData changes
  useEffect(() => {
    if (capData.length === 0) return;

    const initialValues: Record<string, Record<string, any>> = {};

    for (const cap of capData) {
      if (!cap.interfaceSpec) continue;

      const capKey = cap.ident.package;
      initialValues[capKey] = {};

      for (const cls of cap.interfaceSpec.classes) {
        if (!cls.config || !cls.config.fields || cls.config.fields.length === 0) continue;

        const classValues: Record<string, any> = {};
        for (const field of cls.config.fields) {
          // Use default from the binary's ConfiguredCapability if available
          const binaryDefault = cap.defaults[cls.name];
          if (binaryDefault && binaryDefault[field.name] !== undefined) {
            classValues[field.name] = binaryDefault[field.name];
          } else {
            classValues[field.name] = getInitialValueForType(field.data_type);
          }
        }
        initialValues[capKey][cls.name] = classValues;
      }
    }

    setFormValues(initialValues);
  }, [capData]);

  // Propagate changes to parent
  useEffect(() => {
    const configurations: ConfiguredCapability[] = [];

    for (const cap of capData) {
      if (!cap.interfaceSpec) continue;
      const capKey = cap.ident.package;
      const capValues = formValues[capKey];
      if (!capValues) continue;

      const classes: Record<string, any> = {};
      let hasAnyConfig = false;

      for (const cls of cap.interfaceSpec.classes) {
        if (!cls.config || !cls.config.fields || cls.config.fields.length === 0) continue;

        const classValues = capValues[cls.name];
        if (!classValues) {
          classes[cls.name] = null;
          continue;
        }

        // Build the config JSON from form values
        const configObj: Record<string, any> = {};
        for (const field of cls.config.fields) {
          const val = classValues[field.name];
          configObj[field.name] = buildPayloadForType(val, field.data_type, field.name);
        }
        classes[cls.name] = configObj;
        hasAnyConfig = true;
      }

      if (hasAnyConfig) {
        configurations.push({
          author: cap.ident.author,
          package: cap.ident.package,
          version: cap.ident.version,
          configuration: { classes },
        });
      }
    }

    onChange(configurations);
  }, [formValues, capData]);

  const handleFieldChange = useCallback(
    (capPackage: string, className: string, fieldName: string, value: any) => {
      setFormValues((prev) => {
        const capValues = { ...(prev[capPackage] || {}) };
        const classValues = { ...(capValues[className] || {}) };
        classValues[fieldName] = value;
        capValues[className] = classValues;
        return { ...prev, [capPackage]: capValues };
      });
    },
    []
  );

  // Check if any capability has configurable classes
  const hasConfigurableClasses = capData.some(
    (cap) =>
      cap.interfaceSpec &&
      cap.interfaceSpec.classes.some(
        (cls) => cls.config && cls.config.fields && cls.config.fields.length > 0
      )
  );

  if (capData.length === 0 || !hasConfigurableClasses) {
    return null;
  }

  const renderConfigField = (
    capPackage: string,
    className: string,
    field: any
  ) => {
    const value = formValues[capPackage]?.[className]?.[field.name];
    const fieldId = `config-${capPackage}-${className}-${field.name}`;
    const isGroup = isGroupType(field.data_type);
    const isComplex = !isGroup && isComplexType(field.data_type);
    const isBool =
      !isGroup &&
      !isComplex &&
      typeof field.data_type === "object" &&
      field.data_type.PrimitiveScalar === "Bool";
    const isNum =
      !isGroup &&
      !isComplex &&
      typeof field.data_type === "object" &&
      field.data_type.PrimitiveScalar &&
      field.data_type.PrimitiveScalar !== "Bool";

    if (isGroup) {
      // For Group types, render as JSON textarea
      return (
        <div key={fieldId} style={{ marginBottom: "12px" }}>
          <label
            htmlFor={fieldId}
            style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "13px" }}
          >
            {field.name}{" "}
            <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
              ({renderType(field.data_type)}){field.nullable ? " - Optional" : " *"}
            </span>
          </label>
          {field.documentation && (
            <p style={{ margin: "2px 0 6px 0", fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic" }}>
              {field.documentation}
            </p>
          )}
          <textarea
            id={fieldId}
            rows={3}
            value={
              value !== undefined && value !== null
                ? typeof value === "string"
                  ? value
                  : JSON.stringify(value, null, 2)
                : ""
            }
            onChange={(e) => {
              try {
                const parsed = JSON.parse(e.target.value);
                handleFieldChange(capPackage, className, field.name, parsed);
              } catch {
                handleFieldChange(capPackage, className, field.name, e.target.value);
              }
            }}
            placeholder={`JSON for ${renderType(field.data_type)}`}
          />
        </div>
      );
    }

    if (isComplex) {
      return (
        <div key={fieldId} style={{ marginBottom: "12px" }}>
          <label
            htmlFor={fieldId}
            style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "13px" }}
          >
            {field.name}{" "}
            <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
              ({renderType(field.data_type)}){field.nullable ? " - Optional" : " *"}
            </span>
          </label>
          {field.documentation && (
            <p style={{ margin: "2px 0 6px 0", fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic" }}>
              {field.documentation}
            </p>
          )}
          <textarea
            id={fieldId}
            rows={3}
            value={value !== undefined && value !== null ? value : ""}
            onChange={(e) => handleFieldChange(capPackage, className, field.name, e.target.value)}
            placeholder={`Enter JSON for ${renderType(field.data_type)}`}
          />
        </div>
      );
    }

    if (isBool) {
      return (
        <div
          key={fieldId}
          style={{ display: "flex", alignItems: "center", gap: "8px", padding: "4px 0", marginBottom: "8px" }}
        >
          <input
            type="checkbox"
            id={fieldId}
            checked={!!value}
            onChange={(e) => handleFieldChange(capPackage, className, field.name, e.target.checked)}
            style={{ width: "18px", height: "18px", accentColor: "var(--color-primary)", cursor: "pointer" }}
          />
          <label htmlFor={fieldId} style={{ margin: 0, cursor: "pointer", userSelect: "none", fontWeight: 600, fontSize: "13px" }}>
            {field.name}{" "}
            <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
              (Bool){field.nullable ? " - Optional" : " *"}
            </span>
          </label>
          {field.documentation && (
            <span style={{ fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic", marginLeft: "8px" }}>
              {field.documentation}
            </span>
          )}
        </div>
      );
    }

    if (isNum) {
      return (
        <div key={fieldId} style={{ marginBottom: "12px" }}>
          <label
            htmlFor={fieldId}
            style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "13px" }}
          >
            {field.name}{" "}
            <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
              ({renderType(field.data_type)}){field.nullable ? " - Optional" : " *"}
            </span>
          </label>
          {field.documentation && (
            <p style={{ margin: "2px 0 6px 0", fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic" }}>
              {field.documentation}
            </p>
          )}
          <input
            type="number"
            id={fieldId}
            value={value !== undefined && value !== null ? value : ""}
            onChange={(e) => handleFieldChange(capPackage, className, field.name, e.target.value)}
            placeholder={field.nullable ? "Optional" : "Required number"}
          />
        </div>
      );
    }

    // Default: text input
    return (
      <div key={fieldId} style={{ marginBottom: "12px" }}>
        <label
          htmlFor={fieldId}
          style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "13px" }}
        >
          {field.name}{" "}
          <span className="text-muted" style={{ fontSize: "11px", fontWeight: "normal" }}>
            ({renderType(field.data_type)}){field.nullable ? " - Optional" : " *"}
          </span>
        </label>
        {field.documentation && (
          <p style={{ margin: "2px 0 6px 0", fontSize: "12px", color: "var(--text-muted)", fontStyle: "italic" }}>
            {field.documentation}
          </p>
        )}
        <input
          type="text"
          id={fieldId}
          value={value !== undefined && value !== null ? value : ""}
          onChange={(e) => handleFieldChange(capPackage, className, field.name, e.target.value)}
          placeholder={field.nullable ? "Optional" : "Required"}
        />
      </div>
    );
  };

  return (
    <div style={{ marginTop: "16px" }}>
      <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "12px", color: "var(--text-primary)" }}>
        Capability Configuration
      </h4>

      {capData.map((cap) => {
        if (cap.loading) {
          return (
            <div key={cap.ident.package} style={{ padding: "12px", color: "var(--text-muted)", fontSize: "13px" }}>
              Loading {cap.ident.package} interface...
            </div>
          );
        }

        if (cap.error) {
          return (
            <div
              key={cap.ident.package}
              style={{
                padding: "10px 14px",
                marginBottom: "10px",
                borderRadius: "var(--border-radius)",
                backgroundColor: "rgba(239, 68, 68, 0.08)",
                border: "1px solid rgba(239, 68, 68, 0.2)",
                color: "#f87171",
                fontSize: "13px",
              }}
            >
              Failed to load {cap.ident.author}/{cap.ident.package}: {cap.error}
            </div>
          );
        }

        if (!cap.interfaceSpec) return null;

        const configurableClasses = cap.interfaceSpec.classes.filter(
          (cls) => cls.config && cls.config.fields && cls.config.fields.length > 0
        );

        if (configurableClasses.length === 0) return null;

        return (
          <div
            key={cap.ident.package}
            style={{
              border: "1px solid var(--bg-card-border)",
              borderRadius: "var(--border-radius)",
              padding: "16px",
              marginBottom: "12px",
              backgroundColor: "rgba(255, 255, 255, 0.01)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "12px" }}>
              <span
                className="cap-pill"
                style={{
                  padding: "3px 8px",
                  fontSize: "11px",
                  fontWeight: 600,
                  borderRadius: "4px",
                  backgroundColor: "rgba(99, 102, 241, 0.12)",
                  color: "var(--color-primary)",
                  border: "1px solid rgba(99, 102, 241, 0.25)",
                }}
              >
                {cap.ident.package}
              </span>
              <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                {cap.ident.author}/{cap.ident.package}@{cap.ident.version}
              </span>
            </div>

            {configurableClasses.map((cls) => (
              <div key={cls.name} style={{ marginBottom: "10px" }}>
                <h5
                  style={{
                    fontSize: "13px",
                    fontWeight: 600,
                    marginBottom: "8px",
                    paddingBottom: "4px",
                    borderBottom: "1px solid var(--bg-card-border)",
                    color: "var(--text-secondary)",
                  }}
                >
                  {cls.name}
                  {cls.description && (
                    <span
                      style={{
                        fontWeight: "normal",
                        fontSize: "12px",
                        color: "var(--text-muted)",
                        marginLeft: "10px",
                      }}
                    >
                      — {cls.description}
                    </span>
                  )}
                </h5>
                <div style={{ paddingLeft: "8px" }}>
                  {cls.config!.fields.map((field) =>
                    renderConfigField(cap.ident.package, cls.name, field)
                  )}
                </div>
              </div>
            ))}
          </div>
        );
      })}
    </div>
  );
}
