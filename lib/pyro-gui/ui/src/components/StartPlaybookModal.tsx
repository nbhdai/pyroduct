import React, { useState, useEffect } from "react";

interface StartPlaybookModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (params: {
    name: string;
    configPath: string;
    socketPath: string | null;
    inputDir: string | null;
    outputDir: string | null;
  }) => void;
}

export function StartPlaybookModal({ isOpen, onClose, onSubmit }: StartPlaybookModalProps) {
  const [name, setName] = useState("");
  const [configPath, setConfigPath] = useState("");
  const [socketPath, setSocketPath] = useState("");
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setName("");
      setConfigPath("");
      setSocketPath("");
      setInputDir("");
      setOutputDir("");
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      name,
      configPath,
      socketPath: socketPath.trim() || null,
      inputDir: inputDir.trim() || null,
      outputDir: outputDir.trim() || null,
    });
  };

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  return (
    <div className="modal-overlay active" onClick={handleOverlayClick}>
      <div className="modal">
        <div className="modal-header">
          <h3>Start New Playbook</h3>
          <button className="modal-close" onClick={onClose}>
            &times;
          </button>
        </div>
        <div className="modal-body">
          <form onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="playbook-name">Playbook Name *</label>
              <input
                type="text"
                id="playbook-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                placeholder="e.g. my-agent-run"
              />
            </div>
            <div className="form-group">
              <label htmlFor="playbook-config-path">Config File Path (.toml / .yaml) *</label>
              <input
                type="text"
                id="playbook-config-path"
                value={configPath}
                onChange={(e) => setConfigPath(e.target.value)}
                required
                placeholder="e.g. /path/to/playbook.toml"
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
              <label htmlFor="playbook-input-dir">Input Directory (Optional)</label>
              <input
                type="text"
                id="playbook-input-dir"
                value={inputDir}
                onChange={(e) => setInputDir(e.target.value)}
                placeholder="Defaults to playbook workspace input/"
              />
            </div>
            <div className="form-group">
              <label htmlFor="playbook-output-dir">Output Directory (Optional)</label>
              <input
                type="text"
                id="playbook-output-dir"
                value={outputDir}
                onChange={(e) => setOutputDir(e.target.value)}
                placeholder="Defaults to playbook workspace output/"
              />
            </div>
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
