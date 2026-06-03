import React, { useState, useEffect } from "react";

interface CallPlaybookModalProps {
  isOpen: boolean;
  playbookName: string;
  onClose: () => void;
  onSubmit: (name: string, payload: any) => Promise<any>;
}

const DEFAULT_PAYLOAD = `{
  "fields": [
    {"name": "input_text", "value": "hello"}
  ]
}`;

export function CallPlaybookModal({ isOpen, playbookName, onClose, onSubmit }: CallPlaybookModalProps) {
  const [payloadStr, setPayloadStr] = useState(DEFAULT_PAYLOAD);
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  // Reset form when modal opens or target changes
  useEffect(() => {
    if (isOpen) {
      setPayloadStr(DEFAULT_PAYLOAD);
      setResult(null);
      setSubmitting(false);
    }
  }, [isOpen, playbookName]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    let payload: any;
    try {
      payload = JSON.parse(payloadStr);
    } catch (err: any) {
      alert("Invalid JSON payload: " + err.message);
      return;
    }

    setSubmitting(true);
    setResult(null);
    try {
      const res = await onSubmit(playbookName, payload);
      setResult(JSON.stringify(res, null, 2));
    } catch (err: any) {
      setResult(`Error: ${err}`);
    } finally {
      setSubmitting(false);
    }
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
          <h3>
            Call Playbook: <span>{playbookName}</span>
          </h3>
          <button className="modal-close" onClick={onClose}>
            &times;
          </button>
        </div>
        <div className="modal-body">
          <form onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="call-playbook-payload">Payload (JSON) *</label>
              <textarea
                id="call-playbook-payload"
                rows={8}
                value={payloadStr}
                onChange={(e) => setPayloadStr(e.target.value)}
                required
                placeholder={DEFAULT_PAYLOAD}
              />
            </div>

            <div className="form-actions">
              <button type="button" className="btn btn-secondary modal-close-btn" onClick={onClose}>
                Cancel
              </button>
              <button type="submit" disabled={submitting} className="btn btn-primary">
                {submitting ? "Calling..." : "Send Call"}
              </button>
            </div>
          </form>

          {result !== null && (
            <div className="mt-20">
              <h4>Result</h4>
              <pre className="console-box" style={{ marginTop: "8px", maxHeight: "250px", overflowY: "auto" }}>
                {result}
              </pre>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
