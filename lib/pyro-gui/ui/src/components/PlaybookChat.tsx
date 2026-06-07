import React, { useState, useEffect, useRef } from "react";
import { PlaybookSpec } from "../types";
import { invoke } from "@tauri-apps/api/core";
import { LogViewer } from "./LogViewer";

interface ChatMessage {
  role: string;
  content: string;
}

interface PlaybookChatProps {
  playbookName: string;
  playbookSpec?: PlaybookSpec;
  onSubmit: (name: string, payload: any, sessionId?: number) => Promise<any>;
}

const extractContentFromSuccess = (success: any): string => {
  if (!success) return "";
  if (success.content !== undefined) {
    return typeof success.content === "string" ? success.content : JSON.stringify(success.content);
  }
  if (success.output !== undefined) {
    if (typeof success.output === "object" && success.output !== null) {
      if (success.output.content !== undefined) {
        return typeof success.output.content === "string" ? success.output.content : JSON.stringify(success.output.content);
      }
      return JSON.stringify(success.output);
    }
    return typeof success.output === "string" ? success.output : JSON.stringify(success.output);
  }
  if (success.message !== undefined) {
    if (typeof success.message === "object" && success.message !== null) {
      if (success.message.content !== undefined) {
        return typeof success.message.content === "string" ? success.message.content : JSON.stringify(success.message.content);
      }
      return JSON.stringify(success.message);
    }
    return typeof success.message === "string" ? success.message : JSON.stringify(success.message);
  }
  // Fallback: get first field value that is not "role"
  const keys = Object.keys(success);
  const contentKey = keys.find(k => k !== "role") || keys[0];
  if (contentKey !== undefined) {
    const val = success[contentKey];
    return typeof val === "string" ? val : JSON.stringify(val);
  }
  return JSON.stringify(success);
};

export function PlaybookChat({ playbookName, onSubmit }: PlaybookChatProps) {
  const [sessionId, setSessionId] = useState<number | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputText, setInputText] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sessions, setSessions] = useState<Array<{ session_id: number, status: string }>>([]);
  const [currentLogs, setCurrentLogs] = useState<any | null>(null);
  const [showLogsModal, setShowLogsModal] = useState(false);
  const [loadingSession, setLoadingSession] = useState(false);
  
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, submitting]);

  const fetchSessionsList = async () => {
    try {
      const res = await invoke("list_sessions", { playbookName }) as Array<{ session_id: number, status: string }>;
      setSessions(res || []);
    } catch (err) {
      console.error("Failed to load sessions list:", err);
    }
  };

  // Reset chat when playbook changes
  useEffect(() => {
    setSessionId(null);
    setMessages([]);
    setInputText("");
    setError(null);
    setSubmitting(false);
    setCurrentLogs(null);
    fetchSessionsList();
  }, [playbookName]);

  const handleSelectSession = async (sid: number | null) => {
    if (sid === null) {
      setSessionId(null);
      setMessages([]);
      setCurrentLogs(null);
      setError(null);
      return;
    }
    
    setLoadingSession(true);
    setError(null);
    try {
      const record = await invoke("get_playbook_execution_record", {
        playbookName,
        id: sid,
      }) as any;
      
      const updatedHistory = rebuildMessagesFromRecord(record);
      setMessages(updatedHistory);
      setSessionId(sid);
      
      // Extract logs from record if available
      const recordInner = record.Normal || record.Session || record.SessionDiff;
      const successData = recordInner?.Success || recordInner?.Failure;
      if (successData && successData.logs) {
        setCurrentLogs(successData.logs);
      } else {
        setCurrentLogs(null);
      }
    } catch (err: any) {
      setError(`Failed to load session: ${err}`);
      console.error(err);
    } finally {
      setLoadingSession(false);
    }
  };

  const rebuildMessagesFromRecord = (res: any) => {
    const inner = res.Normal || res.Session || res.SessionDiff;
    if (!inner) return [];
    const list: ChatMessage[] = [];
    
    if (res.Session) {
      const successData = res.Session.Success || res.Session.Failure;
      if (successData) {
        // 1. Map prior history
        const prior = successData.prior || [];
        prior.forEach((row: any) => {
          if (row.role) {
            list.push({
              role: row.role,
              content: extractContentFromSuccess(row),
            });
          } else if (row.input) {
            list.push({
              role: row.input.role || "user",
              content: row.input.content || extractContentFromSuccess(row.input),
            });
          } else if (row.output) {
            list.push({
              role: row.output.role || "assistant",
              content: row.output.content || extractContentFromSuccess(row.output),
            });
          } else {
            const text = extractContentFromSuccess(row);
            list.push({
              role: list.length % 2 === 0 ? "user" : "assistant",
              content: text,
            });
          }
        });
        
        // 2. Map current input
        if (successData.input) {
          list.push({
            role: successData.input.role || "user",
            content: successData.input.content || extractContentFromSuccess(successData.input),
          });
        }
        
        // 3. Map current success / failure
        if (res.Session.Success) {
          const succ = res.Session.Success.success;
          if (succ) {
            list.push({
              role: succ.role || succ.output?.role || "assistant",
              content: extractContentFromSuccess(succ),
            });
          }
        } else if (res.Session.Failure) {
          const failMsg = typeof successData.failure === "string" 
            ? successData.failure 
            : (successData.failure?.Ok?.message || JSON.stringify(successData.failure));
          list.push({
            role: "system",
            content: `Error: ${failMsg}`,
          });
        }
      }
    } else if (res.SessionDiff) {
      const successData = res.SessionDiff.Success || res.SessionDiff.Failure;
      if (successData) {
        const priorInput = successData.prior_input || [];
        const priorOutput = successData.prior_output || [];
        const maxLen = Math.max(priorInput.length, priorOutput.length);
        for (let i = 0; i < maxLen; i++) {
          if (i < priorInput.length) {
            const row = priorInput[i];
            list.push({
              role: row.role || row.input?.role || "user",
              content: row.content || row.input?.content || extractContentFromSuccess(row),
            });
          }
          if (i < priorOutput.length) {
            const row = priorOutput[i];
            list.push({
              role: row.role || row.output?.role || "assistant",
              content: row.content || row.output?.content || extractContentFromSuccess(row),
            });
          }
        }
        
        if (successData.input) {
          list.push({
            role: successData.input.role || "user",
            content: successData.input.content || extractContentFromSuccess(successData.input),
          });
        }
        
        if (res.SessionDiff.Success) {
          const succ = res.SessionDiff.Success.success;
          if (succ) {
            list.push({
              role: succ.role || succ.output?.role || "assistant",
              content: extractContentFromSuccess(succ),
            });
          }
        } else if (res.SessionDiff.Failure) {
          const failMsg = typeof successData.failure === "string" 
            ? successData.failure 
            : (successData.failure?.Ok?.message || JSON.stringify(successData.failure));
          list.push({
            role: "system",
            content: `Error: ${failMsg}`,
          });
        }
      }
    }
    return list;
  };

  const handleSend = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!inputText.trim() || submitting) return;

    const messageText = inputText.trim();
    setInputText("");
    setError(null);
    setSubmitting(true);

    // Optimistically add user message
    setMessages((prev) => [...prev, { role: "user", content: messageText }]);

    try {
      const payload = {
        role: "user",
        content: messageText,
      };

      const res = await onSubmit(playbookName, payload, sessionId !== null ? sessionId : undefined);
      
      const record = res.Normal || res.Session || res.SessionDiff;
      if (record && record.Success) {
        setSessionId(record.Success.row_index);
      } else if (record && record.Failure) {
        setSessionId(record.Failure.row_index);
      }

      // Extract and update logs
      const successData = record?.Success || record?.Failure;
      if (successData && successData.logs) {
        setCurrentLogs(successData.logs);
      } else {
        setCurrentLogs(null);
      }

      // Refresh the session list in the dropdown
      fetchSessionsList();

      const updatedHistory = rebuildMessagesFromRecord(res);
      if (updatedHistory.length > 0) {
        setMessages(updatedHistory);
      } else {
        // Fallback: manually push assistant reply if rebuild returned empty
        let reply = "No response";
        if (record && record.Success) {
          reply = extractContentFromSuccess(record.Success.success);
        } else if (record && record.Failure) {
          reply = `Error: ${JSON.stringify(record.Failure.failure)}`;
        }
        setMessages((prev) => [...prev, { role: "assistant", content: reply }]);
      }
    } catch (err: any) {
      setError(String(err));
      setMessages((prev) => [...prev, { role: "system", content: `Connection error: ${err}` }]);
    } finally {
      setSubmitting(false);
    }
  };

  const handleReset = () => {
    handleSelectSession(null);
  };

  return (
    <div className="card" style={{ display: "flex", flexDirection: "column", minHeight: "500px", padding: 0, overflow: "hidden" }}>
      {/* Chat Header Status */}
      <div style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        padding: "16px 24px",
        borderBottom: "1px solid var(--bg-card-border)",
        backgroundColor: "rgba(255, 255, 255, 0.01)",
        flexWrap: "wrap",
        gap: "12px"
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <div style={{
            width: "8px",
            height: "8px",
            borderRadius: "50%",
            backgroundColor: sessionId !== null ? "var(--color-success)" : "var(--color-primary)",
            boxShadow: sessionId !== null ? "0 0 8px var(--color-success)" : "0 0 8px var(--color-primary)"
          }} />
          <span style={{ fontSize: "14px", fontWeight: 600, display: "flex", alignItems: "center", gap: "8px" }}>
            {sessionId !== null ? `Session #${sessionId}` : "New Conversation"}
            {loadingSession && (
              <span className="spinner" style={{ width: "12px", height: "12px", borderWidth: "2px", margin: 0 }} />
            )}
          </span>
        </div>
        
        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          {/* Session Selector */}
          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>Resume:</span>
            <select
              value={sessionId !== null ? String(sessionId) : "new"}
              onChange={(e) => {
                const val = e.target.value;
                handleSelectSession(val === "new" ? null : parseInt(val));
              }}
              style={{
                padding: "6px 12px",
                borderRadius: "6px",
                backgroundColor: "rgba(255, 255, 255, 0.03)",
                border: "1px solid var(--bg-card-border)",
                color: "var(--text-main)",
                fontSize: "12px",
                outline: "none",
                cursor: "pointer"
              }}
            >
              <option value="new">-- New Chat --</option>
              {sessions.map((s) => (
                <option key={s.session_id} value={s.session_id}>
                  Session #{s.session_id} ({s.status})
                </option>
              ))}
            </select>
          </div>

          {currentLogs && (
            <button
              onClick={() => setShowLogsModal(true)}
              className="btn btn-secondary btn-sm"
              style={{ padding: "6px 12px", fontSize: "12px", display: "flex", alignItems: "center", gap: "4px" }}
            >
              📄 View Logs
            </button>
          )}

          <button onClick={handleReset} className="btn btn-secondary btn-sm" style={{ padding: "6px 12px", fontSize: "12px" }}>
            New Chat
          </button>
        </div>
      </div>

      {/* Messages Thread Container */}
      <div style={{
        flexGrow: 1,
        padding: "24px",
        overflowY: "auto",
        maxHeight: "400px",
        display: "flex",
        flexDirection: "column",
        gap: "16px",
        backgroundColor: "#0d0e12"
      }}>
        {messages.length === 0 ? (
          <div style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            height: "250px",
            color: "var(--text-muted)",
            textAlign: "center"
          }}>
            <span style={{ fontSize: "36px", marginBottom: "12px", opacity: 0.3 }}>💬</span>
            <p style={{ fontSize: "14px", fontWeight: 500, margin: 0 }}>Start a new conversation with {playbookName}.</p>
            <p style={{ fontSize: "12px", opacity: 0.7, marginTop: "4px" }}>The playbook will process messages and maintain context.</p>
          </div>
        ) : (
          messages.map((msg, index) => {
            const isUser = msg.role === "user";
            const isSystem = msg.role === "system";
            
            return (
              <div
                key={index}
                style={{
                  display: "flex",
                  justifyContent: isUser ? "flex-end" : "flex-start",
                  width: "100%"
                }}
              >
                <div
                  style={{
                    maxWidth: "75%",
                    padding: "12px 16px",
                    borderRadius: "12px",
                    fontSize: "14px",
                    lineHeight: "1.5",
                    wordBreak: "break-word",
                    backgroundColor: isUser 
                      ? "var(--color-primary)" 
                      : isSystem 
                        ? "rgba(239, 68, 68, 0.08)"
                        : "rgba(255, 255, 255, 0.04)",
                    color: isUser ? "#ffffff" : isSystem ? "#f87171" : "var(--text-main)",
                    border: isUser 
                      ? "none" 
                      : isSystem 
                        ? "1px solid rgba(239, 68, 68, 0.2)"
                        : "1px solid var(--bg-card-border)",
                    boxShadow: isUser ? "0 4px 12px rgba(255, 92, 0, 0.15)" : "none",
                    alignSelf: isUser ? "flex-end" : "flex-start"
                  }}
                >
                  {/* Message Label for non-user */}
                  {!isUser && (
                    <div style={{
                      fontSize: "10px",
                      fontWeight: 700,
                      textTransform: "uppercase",
                      letterSpacing: "0.05em",
                      color: isSystem ? "#ef4444" : "var(--color-primary)",
                      marginBottom: "4px"
                    }}>
                      {msg.role}
                    </div>
                  )}
                  <div style={{ whiteSpace: "pre-wrap" }}>{msg.content}</div>
                </div>
              </div>
            );
          })
        )}

        {/* Thinking / Spinner State */}
        {submitting && (
          <div style={{ display: "flex", justifyContent: "flex-start", width: "100%" }}>
            <div style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              padding: "12px 16px",
              borderRadius: "12px",
              backgroundColor: "rgba(255, 255, 255, 0.03)",
              border: "1px solid var(--bg-card-border)",
              color: "var(--text-muted)",
              fontSize: "13px"
            }}>
              <span className="thinking-dots" style={{ display: "flex", gap: "3px" }}>
                <span className="dot" style={{ width: "6px", height: "6px", backgroundColor: "var(--color-primary)", borderRadius: "50%", display: "inline-block", animation: "bounce 1.4s infinite ease-in-out both", animationDelay: "-0.32s" }}></span>
                <span className="dot" style={{ width: "6px", height: "6px", backgroundColor: "var(--color-primary)", borderRadius: "50%", display: "inline-block", animation: "bounce 1.4s infinite ease-in-out both", animationDelay: "-0.16s" }}></span>
                <span className="dot" style={{ width: "6px", height: "6px", backgroundColor: "var(--color-primary)", borderRadius: "50%", display: "inline-block", animation: "bounce 1.4s infinite ease-in-out both" }}></span>
              </span>
              <span>Assistant is thinking...</span>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input area at bottom */}
      <div style={{
        padding: "16px 24px",
        borderTop: "1px solid var(--bg-card-border)",
        backgroundColor: "rgba(255, 255, 255, 0.01)"
      }}>
        <form onSubmit={handleSend} style={{ display: "flex", gap: "12px" }}>
          <input
            type="text"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            disabled={submitting}
            placeholder={submitting ? "Please wait..." : "Type your message..."}
            style={{
              flexGrow: 1,
              padding: "12px 16px",
              borderRadius: "8px",
              backgroundColor: "rgba(255, 255, 255, 0.03)",
              border: "1px solid var(--bg-card-border)",
              color: "var(--text-main)",
              fontFamily: "inherit",
              fontSize: "14px",
              outline: "none"
            }}
          />
          <button
            type="submit"
            disabled={submitting || !inputText.trim()}
            className="btn btn-primary"
            style={{
              padding: "0 24px",
              height: "46px",
              borderRadius: "8px",
              fontSize: "14px",
              fontWeight: 600
            }}
          >
            {submitting ? "Sending..." : "Send"}
          </button>
        </form>
        {error && (
          <div style={{ color: "#ef4444", fontSize: "12px", marginTop: "8px", paddingLeft: "4px" }}>
            ⚠️ {error}
          </div>
        )}
      </div>

      {/* Logs Modal */}
      {showLogsModal && currentLogs && (
        <div className="modal-overlay active" onClick={() => setShowLogsModal(false)}>
          <div className="modal modal-lg" onClick={(e) => e.stopPropagation()} style={{ maxWidth: "800px" }}>
            <div className="modal-header">
              <h3>Session Logs (Session #{sessionId})</h3>
              <button className="modal-close" onClick={() => setShowLogsModal(false)}>
                &times;
              </button>
            </div>
            <div className="modal-body" style={{ maxHeight: "450px", overflowY: "auto" }}>
              <LogViewer logs={currentLogs} />
            </div>
            <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end", marginTop: "15px", borderTop: "1px solid var(--bg-card-border)", paddingTop: "15px" }}>
              <button className="btn btn-secondary" onClick={() => setShowLogsModal(false)}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      <style>{`
        @keyframes bounce {
          0%, 80%, 100% { transform: scale(0); }
          40% { transform: scale(1.0); }
        }
      `}</style>
    </div>
  );
}
