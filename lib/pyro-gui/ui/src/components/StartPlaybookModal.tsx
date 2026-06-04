import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PlaybookItem {
  author: string;
  name: string;
  version: string;
}

interface StartPlaybookModalProps {
  isOpen: boolean;
  availablePlaybooks: PlaybookItem[];
  onClose: () => void;
  onSubmit: (params: {
    name: string;
    configPath?: string;
    playbookIdent?: { author: string; package: string; version: string };
    remote?: Array<{
      capability: { author: string; package: string; version: string };
      address: { tcp: string } | { unix: string };
    }>;
    walCapacity?: number;
    successLogRetentionSecs?: number;
    errorLogRetentionSecs?: number;
    socketPath?: string | null;
    inputDir?: string | null;
    outputDir?: string | null;
  }) => void;
}

export function StartPlaybookModal({
  isOpen,
  availablePlaybooks,
  onClose,
  onSubmit,
}: StartPlaybookModalProps) {
  const [selectedPlaybookKey, setSelectedPlaybookKey] = useState("");
  const [customConfigPath, setCustomConfigPath] = useState("");
  const [name, setName] = useState("");
  const [socketPath, setSocketPath] = useState("");
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  
  // PipelineConfig advanced options
  const [walCapacity, setWalCapacity] = useState<number>(1000);
  const [successLogRetentionSecs, setSuccessLogRetentionSecs] = useState<number>(3600);
  const [errorLogRetentionSecs, setErrorLogRetentionSecs] = useState<number>(604800);
  
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [requiredCapabilities, setRequiredCapabilities] = useState<any[]>([]);
  const [remoteConfig, setRemoteConfig] = useState<Record<string, { type: "tcp" | "unix"; address: string }>>({});

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setName("");
      setCustomConfigPath("");
      setSocketPath("");
      setInputDir("");
      setOutputDir("");
      setWalCapacity(1000);
      setSuccessLogRetentionSecs(3600);
      setErrorLogRetentionSecs(604800);
      setShowAdvanced(false);
      setRequiredCapabilities([]);
      setRemoteConfig({});
      
      // Default to first available playbook, or custom config if empty
      if (availablePlaybooks.length > 0) {
        const pb = availablePlaybooks[0];
        const key = `${pb.author}/${pb.name}@${pb.version}`;
        setSelectedPlaybookKey(key);
        setName(pb.name);
        fetchPlaybookCapabilities(pb.author, pb.name, pb.version);
      } else {
        setSelectedPlaybookKey("__custom__");
      }
    }
  }, [isOpen, availablePlaybooks]);

  const fetchPlaybookCapabilities = async (author: string, packageName: string, version: string) => {
    try {
      const spec: any = await invoke("get_playbook_spec", { author, name: packageName, version });
      if (spec && spec.capabilities) {
        setRequiredCapabilities(spec.capabilities);
        // Initialize empty remote config for each required capability
        const initialRemotes: Record<string, { type: "disabled" | "tcp" | "unix"; address: string }> = {};
        spec.capabilities.forEach((cap: any) => {
          const capKey = `${cap.author}/${cap.package}@${cap.version}`;
          initialRemotes[capKey] = { type: "disabled", address: "" };
        });
        setRemoteConfig(initialRemotes);
      } else {
        setRequiredCapabilities([]);
      }
    } catch (err) {
      console.error("Failed to load playbook capabilities spec:", err);
      setRequiredCapabilities([]);
    }
  };

  const handlePlaybookChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const key = e.target.value;
    setSelectedPlaybookKey(key);
    
    if (key === "__custom__") {
      setRequiredCapabilities([]);
      setName("");
    } else {
      const selected = availablePlaybooks.find(
        (pb) => `${pb.author}/${pb.name}@${pb.version}` === key
      );
      if (selected) {
        setName(selected.name);
        fetchPlaybookCapabilities(selected.author, selected.name, selected.version);
      }
    }
  };

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    const isCustom = selectedPlaybookKey === "__custom__";
    const selected = isCustom
      ? null
      : availablePlaybooks.find(
          (pb) => `${pb.author}/${pb.name}@${pb.version}` === selectedPlaybookKey
        );

    // Format remotes list
    const formattedRemotes = Object.entries(remoteConfig)
      .filter(([_, cfg]) => cfg.type !== "disabled" && cfg.address.trim() !== "")
      .map(([capKey, cfg]) => {
        const [authorAndPack, version] = capKey.split("@");
        const [author, packageName] = authorAndPack.split("/");
        return {
          capability: {
            author,
            package: packageName,
            version,
          },
          address: cfg.type === "tcp" ? { tcp: cfg.address.trim() } : { unix: cfg.address.trim() },
        };
      });

    onSubmit({
      name: name.trim() || (selected ? selected.name : "playbook-run"),
      configPath: isCustom ? customConfigPath.trim() : undefined,
      playbookIdent: selected
        ? {
            author: selected.author,
            package: selected.name,
            version: selected.version,
          }
        : undefined,
      remote: formattedRemotes.length > 0 ? formattedRemotes : undefined,
      walCapacity,
      successLogRetentionSecs,
      errorLogRetentionSecs,
      socketPath: socketPath.trim() || null,
      inputDir: inputDir.trim() || null,
      outputDir: outputDir.trim() || null,
    });
  };

  const handleRemoteTypeChange = (capKey: string, type: "tcp" | "unix") => {
    setRemoteConfig((prev) => ({
      ...prev,
      [capKey]: {
        ...prev[capKey],
        type,
      },
    }));
  };

  const handleRemoteAddressChange = (capKey: string, address: string) => {
    setRemoteConfig((prev) => ({
      ...prev,
      [capKey]: {
        ...prev[capKey],
        address,
      },
    }));
  };

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  return (
    <div className="modal-overlay active" onClick={handleOverlayClick}>
      <div className="modal modal-lg">
        <div className="modal-header">
          <h3>Start New Playbook</h3>
          <button className="modal-close" onClick={onClose}>
            &times;
          </button>
        </div>
        <div className="modal-body">
          <form onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="playbook-select">Playbook Repository *</label>
              <select
                id="playbook-select"
                value={selectedPlaybookKey}
                onChange={handlePlaybookChange}
                required
              >
                {availablePlaybooks.map((pb, idx) => {
                  const key = `${pb.author}/${pb.name}@${pb.version}`;
                  return (
                    <option key={idx} value={key}>
                      {pb.author}/{pb.name} ({pb.version})
                    </option>
                  );
                })}
                <option value="__custom__">Custom Config File Path...</option>
              </select>
            </div>

            {selectedPlaybookKey === "__custom__" && (
              <div className="form-group">
                <label htmlFor="playbook-config-path">Config File Path (.toml / .yaml) *</label>
                <input
                  type="text"
                  id="playbook-config-path"
                  value={customConfigPath}
                  onChange={(e) => setCustomConfigPath(e.target.value)}
                  required
                  placeholder="e.g. /path/to/playbook.toml"
                />
              </div>
            )}

            <button
              type="button"
              className={`advanced-toggle ${showAdvanced ? "expanded" : ""}`}
              onClick={() => setShowAdvanced(!showAdvanced)}
            >
              Advanced Options
            </button>

            {showAdvanced && (
              <div className="advanced-section">
                <div className="form-group">
                  <label htmlFor="playbook-name">Playbook Worker Name (Optional)</label>
                  <input
                    type="text"
                    id="playbook-name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g. my-agent-run"
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="playbook-socket">Custom Socket Path (Optional)</label>
                  <input
                    type="text"
                    id="playbook-socket"
                    value={socketPath}
                    onChange={(e) => setSocketPath(e.target.value)}
                    placeholder="e.g. /tmp/playbook.sock"
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="playbook-input-dir">Input Directory Override (Optional)</label>
                  <input
                    type="text"
                    id="playbook-input-dir"
                    value={inputDir}
                    onChange={(e) => setInputDir(e.target.value)}
                    placeholder="Defaults to playbook workspace input/"
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="playbook-output-dir">Output Directory Override (Optional)</label>
                  <input
                    type="text"
                    id="playbook-output-dir"
                    value={outputDir}
                    onChange={(e) => setOutputDir(e.target.value)}
                    placeholder="Defaults to playbook workspace output/"
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="playbook-wal-capacity">WAL Capacity (Default: 1000)</label>
                  <input
                    type="number"
                    id="playbook-wal-capacity"
                    value={walCapacity}
                    onChange={(e) => setWalCapacity(parseInt(e.target.value) || 0)}
                    min={1}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="playbook-success-retention">Success Log Retention (Seconds, Default: 3600)</label>
                  <input
                    type="number"
                    id="playbook-success-retention"
                    value={successLogRetentionSecs}
                    onChange={(e) => setSuccessLogRetentionSecs(parseInt(e.target.value) || 0)}
                    min={0}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="playbook-error-retention">Error Log Retention (Seconds, Default: 604800)</label>
                  <input
                    type="number"
                    id="playbook-error-retention"
                    value={errorLogRetentionSecs}
                    onChange={(e) => setErrorLogRetentionSecs(parseInt(e.target.value) || 0)}
                    min={0}
                  />
                </div>

                {requiredCapabilities.length > 0 && (
                  <div className="form-group">
                    <label>Remote Capability Addresses (Optional)</label>
                    {requiredCapabilities.map((cap, idx) => {
                      const capKey = `${cap.author}/${cap.package}@${cap.version}`;
                      const config = remoteConfig[capKey] || { type: "tcp", address: "" };
                      return (
                        <div key={idx} className="capability-config-group">
                          <div className="capability-config-header">
                            <span>{cap.author}/{cap.package}@{cap.version}</span>
                          </div>
                          <div className="capability-config-fields">
                            <select
                              style={config.type === "disabled" ? { gridColumn: "span 2" } : undefined}
                              value={config.type}
                              onChange={(e) => handleRemoteTypeChange(capKey, e.target.value as any)}
                            >
                              <option value="disabled">Disabled (Local Process)</option>
                              <option value="tcp">TCP</option>
                              <option value="unix">UNIX Socket</option>
                            </select>
                            {config.type !== "disabled" && (
                              <input
                                type="text"
                                value={config.address}
                                onChange={(e) => handleRemoteAddressChange(capKey, e.target.value)}
                                placeholder={config.type === "tcp" ? "127.0.0.1:8000" : "/path/to/uds.sock"}
                                required
                              />
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}

            <div className="form-actions">
              <button type="button" className="btn btn-secondary modal-close-btn" onClick={onClose}>
                Cancel
              </button>
              <button type="submit" className="btn btn-success">
                Launch Worker
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
