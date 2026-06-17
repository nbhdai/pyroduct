export interface DaemonStatus {
  status: "online" | "offline" | "error";
  socket_path?: string;
  daemon_name?: string;
  active_workers?: number;
  version?: string;
  message?: string;
  running_playbooks?: Playbook[];
}

export interface Capability {
  author: string;
  name: string;
  version: string;
}

export interface RepoPlaybook {
  author: string;
  name: string;
  version: string;
}

export interface CacheStatus {
  cache_root: string;
  capabilities: Capability[];
  playbooks: RepoPlaybook[];
}

export interface PlaybookCapability {
  author?: string;
  package: string;
  version: string;
}

export interface Playbook {
  name: string;
  config_path: string;
  socket_path?: string;
  active_capabilities?: PlaybookCapability[];
  local_capabilities?: PlaybookCapability[];
  remote_capabilities?: PlaybookCapability[];
  spec?: PlaybookSpec;
  processed_rows?: number;
}

export interface LogEntry {
  time: string;
  message: string;
  type: "system" | "success" | "error" | "command";
}

export interface PyroField {
  name: string;
  documentation?: string;
  data_type: any;
  nullable: boolean;
}

export interface PyroSchema {
  documentation?: string;
  fields: PyroField[];
}

export interface CapabilityFunc {
  name: string;
  description?: string;
  input: PyroSchema;
  output: any;
}

export interface ClassSpec {
  name: string;
  description?: string;
  methods: CapabilityFunc[];
  client?: PyroSchema;
  config?: PyroSchema;
}

export interface InterfaceSpec {
  capability: string;
  description?: string;
  classes: ClassSpec[];
}

export interface ModuleFunc {
  name: string;
  description?: string;
  input: PyroSchema;
  output: PyroSchema;
  kind?: "normal" | "session" | "session_diff";
}

export interface PlaybookSpec {
  ident: {
    author: string;
    package: string;
    version: string;
  };
  hash: string;
  func: ModuleFunc;
  capabilities: {
    author: string;
    package: string;
    version: string;
  }[];
  interconnect: Record<string, {
    author: string;
    package: string;
    version: string;
  }>;
}

export interface PyroductConfig {
  author: string;
  build_slots?: number;
}

export interface DaemonConnection {
  type: "unix" | "tcp";
  path?: string;
  address?: string;
}

export interface GuiSettings {
  selected_daemon?: string;
  daemons: Record<string, DaemonConnection>;
}


