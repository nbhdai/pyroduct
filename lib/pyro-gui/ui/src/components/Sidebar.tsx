import { DaemonStatus } from "../types";

interface SidebarProps {
  activeTab: "dashboard" | "repository" | "playbooks" | "options";
  onTabChange: (tab: "dashboard" | "repository" | "playbooks" | "options") => void;
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
          Dashboard
        </button>
        <button
          className={`nav-btn ${activeTab === "repository" ? "active" : ""}`}
          onClick={() => onTabChange("repository")}
        >
          Repository
        </button>
        <button
          className={`nav-btn ${activeTab === "playbooks" ? "active" : ""}`}
          onClick={() => onTabChange("playbooks")}
        >
          Playbooks
        </button>
        <button
          className={`nav-btn ${activeTab === "options" ? "active" : ""}`}
          onClick={() => onTabChange("options")}
        >
          Options
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
