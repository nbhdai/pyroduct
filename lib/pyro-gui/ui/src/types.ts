export interface DaemonStatus {
  status: "online" | "offline" | "error";
  socket_path?: string;
  active_workers?: number;
  version?: string;
  message?: string;
}

export interface Capability {
  author: string;
  name: string;
  version: string;
}

export interface Module {
  author: string;
  name: string;
  version: string;
}

export interface CacheStatus {
  cache_root: string;
  capabilities: Capability[];
  modules: Module[];
}

export interface PlaybookCapability {
  package: string;
  version: string;
}

export interface Playbook {
  name: string;
  config_path: string;
  socket_path?: string;
  active_capabilities?: PlaybookCapability[];
}

export interface LogEntry {
  time: string;
  message: string;
  type: "system" | "success" | "error" | "command";
}
