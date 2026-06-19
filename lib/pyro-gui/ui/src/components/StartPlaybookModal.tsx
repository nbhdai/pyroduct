import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PlaybookSpec } from "../types";
import { CapabilityConfigForm, ConfiguredCapability } from "./CapabilityConfigForm";

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
    playbookIdent: { author: string; package: string; version: string };
    httpAddress?: string | null;
    socketPath?: string | null;
    inputDir?: string | null;
    outputDir?: string | null;
    pinnedVersion?: string | null;
    configurations?: ConfiguredCapability[];
  }) => void;
}

export function StartPlaybookModal({
  isOpen,
  availablePlaybooks,
  onClose,
  onSubmit,
}: StartPlaybookModalProps) {
  const [selectedPlaybookKey, setSelectedPlaybookKey] = useState("");
  const [name, setName] = useState("");
  const [httpAddress, setHttpAddress] = useState("");
  const [socketPath, setSocketPath] = useState("");
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [pinVersion, setPinVersion] = useState(false);
  
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [showConfig, setShowConfig] = useState(false);

  // PlaybookSpec fetched for the selected playbook (to get capabilities list)
  const [playbookSpec, setPlaybookSpec] = useState<PlaybookSpec | null>(null);
  const [configurations, setConfigurations] = useState<ConfiguredCapability[]>([]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setName("");
      setHttpAddress("");
      setSocketPath("");
      setInputDir("");
      setOutputDir("");
      setPinVersion(false);
      setShowAdvanced(false);
      setShowConfig(false);
      setPlaybookSpec(null);
      setConfigurations([]);
      
      // Default to first available playbook
      if (availablePlaybooks.length > 0) {
        const pb = availablePlaybooks[0];
        const key = `${pb.author}/${pb.name}@${pb.version}`;
        setSelectedPlaybookKey(key);
        setName(pb.name);
      } else {
        setSelectedPlaybookKey("");
      }
    }
  }, [isOpen, availablePlaybooks]);

  // Fetch PlaybookSpec when selection changes
  useEffect(() => {
    if (!selectedPlaybookKey) {
      setPlaybookSpec(null);
      return;
    }

    const selected = availablePlaybooks.find(
      (pb) => `${pb.author}/${pb.name}@${pb.version}` === selectedPlaybookKey
    );
    if (!selected) {
      setPlaybookSpec(null);
      return;
    }

    let active = true;
    invoke("get_playbook_spec", {
      author: selected.author,
      name: selected.name,
      version: selected.version,
    })
      .then((res) => {
        if (active) setPlaybookSpec(res as PlaybookSpec);
      })
      .catch(() => {
        if (active) setPlaybookSpec(null);
      });

    return () => { active = false; };
  }, [selectedPlaybookKey, availablePlaybooks]);

  const handlePlaybookChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const key = e.target.value;
    setSelectedPlaybookKey(key);
    
    const selected = availablePlaybooks.find(
      (pb) => `${pb.author}/${pb.name}@${pb.version}` === key
    );
    if (selected) {
      setName(selected.name);
    }
  };

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    const selected = availablePlaybooks.find(
      (pb) => `${pb.author}/${pb.name}@${pb.version}` === selectedPlaybookKey
    );

    if (!selected) {
      alert("Please select a playbook.");
      return;
    }

    onSubmit({
      name: name.trim() || selected.name,
      playbookIdent: {
        author: selected.author,
        package: selected.name,
        version: selected.version,
      },
      httpAddress: httpAddress.trim() || null,
      socketPath: socketPath.trim() || null,
      inputDir: inputDir.trim() || null,
      outputDir: outputDir.trim() || null,
      pinnedVersion: pinVersion ? selected.version : null,
      configurations: configurations.length > 0 ? configurations : undefined,
    });
  };

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  const hasCapabilities = playbookSpec && playbookSpec.capabilities && playbookSpec.capabilities.length > 0;

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
              </select>
            </div>

            <div className="form-group">
              <label htmlFor="http-address">HTTP Server Address (Optional)</label>
              <input
                type="text"
                id="http-address"
                value={httpAddress}
                onChange={(e) => setHttpAddress(e.target.value)}
                placeholder="e.g. 127.0.0.1:8080"
              />
            </div>

            {/* Capability Configuration Toggle */}
            {hasCapabilities && (
              <>
                <button
                  type="button"
                  className={`advanced-toggle ${showConfig ? "expanded" : ""}`}
                  onClick={() => setShowConfig(!showConfig)}
                >
                  Capability Configuration
                </button>

                {showConfig && (
                  <div className="advanced-section">
                    <CapabilityConfigForm
                      playbookIdent={playbookSpec!.ident}
                      capabilities={playbookSpec!.capabilities}
                      onChange={setConfigurations}
                    />
                  </div>
                )}
              </>
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
                <div className="form-group" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <input
                    type="checkbox"
                    id="pin-version"
                    checked={pinVersion}
                    onChange={(e) => setPinVersion(e.target.checked)}
                    style={{ width: 'auto' }}
                  />
                  <label htmlFor="pin-version" style={{ margin: 0 }}>
                    Pin version (disable auto-updates)
                  </label>
                </div>
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
