import { DaemonStatus } from "../types";

interface SidebarProps {
  activeTab: "dashboard" | "cache" | "playbooks";
  onTabChange: (tab: "dashboard" | "cache" | "playbooks") => void;
  daemonStatus: DaemonStatus;
}

export function Sidebar({ activeTab, onTabChange, daemonStatus }: SidebarProps) {
  const getPulseClass = () => {
    if (daemonStatus.status === "online") return "status-pulse online";
    if (daemonStatus.status === "offline") return "status-pulse offline";
    return "status-pulse checking";
  };

  const getStatusText = () => {
    if (daemonStatus.status === "online") return "Daemon: Online";
    if (daemonStatus.status === "offline") return "Daemon: Offline";
    return "Daemon: Checking...";
  };

  const getSubInfoText = () => {
    if (daemonStatus.status === "online") {
      return `Workers: ${daemonStatus.active_workers ?? 0} | v${daemonStatus.version ?? ""}`;
    }
    return daemonStatus.socket_path ? `socket: ${daemonStatus.socket_path}` : "socket: offline";
  };

  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/icon.png" alt="Pyroduct Logo" className="logo-icon-img" />
        <div className="brand-name">Pyroduct</div>
      </div>

      <nav className="nav-menu">
        <button
          className={`nav-btn ${activeTab === "dashboard" ? "active" : ""}`}
          onClick={() => onTabChange("dashboard")}
        >
          <span className="btn-icon">⚡</span> Dashboard
        </button>
        <button
          className={`nav-btn ${activeTab === "cache" ? "active" : ""}`}
          onClick={() => onTabChange("cache")}
        >
          <span className="btn-icon">📦</span> Cache Explorer
        </button>
        <button
          className={`nav-btn ${activeTab === "playbooks" ? "active" : ""}`}
          onClick={() => onTabChange("playbooks")}
        >
          <span className="btn-icon">⚙️</span> Playbooks
        </button>
      </nav>

      <div className="daemon-quick-status">
        <div className="status-indicator-wrapper">
          <span className={getPulseClass()}></span>
          <span className="status-text">{getStatusText()}</span>
        </div>
        <div className="sub-info" title={getSubInfoText()}>
          {getSubInfoText()}
        </div>
      </div>
    </aside>
  );
}
