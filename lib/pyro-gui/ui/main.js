// Wait for DOM to load
document.addEventListener('DOMContentLoaded', () => {
    const { invoke } = window.__TAURI__.core;

    // UI Elements
    const navButtons = document.querySelectorAll('.nav-btn');
    const tabContents = document.querySelectorAll('.tab-content');
    const subTabButtons = document.querySelectorAll('.sub-tab-btn');
    const subTabContents = document.querySelectorAll('.sub-tab-content');
    const pageTitle = document.getElementById('page-title');
    
    // Status Bar Elements
    const daemonPulse = document.getElementById('daemon-pulse');
    const daemonStatusText = document.getElementById('daemon-status-text');
    const daemonSubInfo = document.getElementById('daemon-sub-info');
    
    // Dashboard Elements
    const infoStatus = document.getElementById('info-status');
    const infoWorkers = document.getElementById('info-workers');
    const infoVersion = document.getElementById('info-version');
    const infoSocket = document.getElementById('info-socket');
    const consoleLogs = document.getElementById('console-logs');
    const quickRefreshBtn = document.getElementById('quick-refresh-btn');
    const quickPurgeBtn = document.getElementById('quick-purge-btn');
    const refreshAllBtn = document.getElementById('refresh-all-btn');

    // Cache Elements
    const cacheRootPath = document.getElementById('cache-root-path');
    const purgeCacheBtn = document.getElementById('purge-cache-btn');
    const capabilitiesTableBody = document.getElementById('capabilities-table-body');
    const modulesTableBody = document.getElementById('modules-table-body');

    // Playbooks Elements
    const playbooksList = document.getElementById('playbooks-list');
    const openStartPlaybookModalBtn = document.getElementById('open-start-playbook-modal');
    
    // Start Playbook Modal Elements
    const startPlaybookModal = document.getElementById('start-playbook-modal');
    const startPlaybookForm = document.getElementById('start-playbook-form');
    
    // Call Playbook Modal Elements
    const callPlaybookModal = document.getElementById('call-playbook-modal');
    const callPlaybookForm = document.getElementById('call-playbook-form');
    const callPlaybookTargetName = document.getElementById('call-playbook-target-name');
    const callPlaybookNameInput = document.getElementById('call-playbook-name');
    const callPlaybookPayloadInput = document.getElementById('call-playbook-payload');
    const callResultContainer = document.getElementById('call-result-container');
    const callResultBox = document.getElementById('call-result-box');

    // Log helper
    function log(message, type = 'system') {
        const line = document.createElement('div');
        line.className = `log-line ${type}`;
        const time = new Date().toLocaleTimeString();
        line.innerText = `[${time}] [${type}] ${message}`;
        consoleLogs.appendChild(line);
        consoleLogs.scrollTop = consoleLogs.scrollHeight;
    }

    // Tab Switching
    navButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const targetTab = btn.getAttribute('data-tab');
            
            navButtons.forEach(b => b.classList.remove('active'));
            tabContents.forEach(tc => tc.classList.remove('active'));
            
            btn.classList.add('active');
            const targetEl = document.getElementById(`tab-${targetTab}`);
            if (targetEl) targetEl.classList.add('active');
            
            // Capitalize title
            pageTitle.innerText = btn.innerText.replace(/[^\w\s]/g, '').trim();
            
            // Specific tab loads
            if (targetTab === 'cache') {
                loadCache();
            } else if (targetTab === 'playbooks') {
                loadPlaybooks();
            }
        });
    });

    // Subtab switching (Cache tabs)
    subTabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const targetSubtab = btn.getAttribute('data-subtab');
            
            subTabButtons.forEach(b => b.classList.remove('active'));
            subTabContents.forEach(stc => stc.classList.remove('active'));
            
            btn.classList.add('active');
            const targetEl = document.getElementById(`subtab-${targetSubtab}`);
            if (targetEl) targetEl.classList.add('active');
        });
    });

    // Modal Control Helper
    function setupModal(modalEl) {
        const closeButtons = modalEl.querySelectorAll('.modal-close, .modal-close-btn');
        closeButtons.forEach(btn => {
            btn.addEventListener('click', () => {
                modalEl.classList.remove('active');
            });
        });
        modalEl.addEventListener('click', (e) => {
            if (e.target === modalEl) {
                modalEl.classList.remove('active');
            }
        });
    }
    setupModal(startPlaybookModal);
    setupModal(callPlaybookModal);

    openStartPlaybookModalBtn.addEventListener('click', () => {
        startPlaybookModal.classList.add('active');
        // Reset form
        startPlaybookForm.reset();
    });

    // Query Status function
    async function queryDaemonStatus() {
        daemonPulse.className = 'status-pulse checking';
        daemonStatusText.innerText = 'Daemon: Querying...';
        
        try {
            log('Querying daemon status...', 'command');
            const res = await invoke('get_daemon_status');
            
            if (res.status === 'online') {
                daemonPulse.className = 'status-pulse online';
                daemonStatusText.innerText = `Daemon: Online`;
                daemonSubInfo.innerText = `Workers: ${res.active_workers} | v${res.version}`;
                
                infoStatus.className = 'value badge badge-online';
                infoStatus.innerText = 'Online';
                infoWorkers.innerText = res.active_workers;
                infoVersion.innerText = res.version;
                infoSocket.innerText = res.socket_path;
                
                log(`Daemon is online. Version ${res.version}. Active workers: ${res.active_workers}.`, 'success');
            } else {
                daemonPulse.className = 'status-pulse offline';
                daemonStatusText.innerText = `Daemon: Offline`;
                daemonSubInfo.innerText = `socket: offline`;
                
                infoStatus.className = 'value badge badge-offline';
                infoStatus.innerText = 'Offline';
                infoWorkers.innerText = '0';
                infoVersion.innerText = '-';
                infoSocket.innerText = res.socket_path || 'unknown';
                
                log(`Daemon offline. ${res.message || ''}`, 'error');
            }
        } catch (err) {
            daemonPulse.className = 'status-pulse offline';
            daemonStatusText.innerText = `Daemon: Error`;
            daemonSubInfo.innerText = `error connecting`;
            
            infoStatus.className = 'value badge badge-offline';
            infoStatus.innerText = 'Error';
            
            log(`Failed to query daemon: ${err}`, 'error');
        }
    }

    // Load Cache Manager state
    async function loadCache() {
        try {
            log('Fetching local cache repository status...', 'command');
            const res = await invoke('get_cache_status');
            
            cacheRootPath.innerText = res.cache_root;
            
            // Render Capabilities
            if (res.capabilities.length === 0) {
                capabilitiesTableBody.innerHTML = `<tr><td colspan="3" class="text-center">No capabilities found in cache.</td></tr>`;
            } else {
                capabilitiesTableBody.innerHTML = res.capabilities.map(cap => `
                    <tr>
                        <td>${cap.author}</td>
                        <td>${cap.name}</td>
                        <td><span class="code-text">${cap.version}</span></td>
                    </tr>
                `).join('');
            }
            
            // Render Modules
            if (res.modules.length === 0) {
                modulesTableBody.innerHTML = `<tr><td colspan="3" class="text-center">No modules found in cache.</td></tr>`;
            } else {
                modulesTableBody.innerHTML = res.modules.map(mod => `
                    <tr>
                        <td>${mod.author}</td>
                        <td>${mod.name}</td>
                        <td><span class="code-text">${mod.version}</span></td>
                    </tr>
                `).join('');
            }
            log(`Loaded ${res.capabilities.length} capabilities and ${res.modules.length} modules from cache.`, 'success');
        } catch (err) {
            log(`Failed to load cache info: ${err}`, 'error');
        }
    }

    // Purge Cache
    async function purgeCache() {
        if (!confirm('Are you sure you want to purge the local cache repository? This deletes cached capability binaries, specifications, and modules.')) {
            return;
        }
        
        try {
            log('Purging cache...', 'command');
            const msg = await invoke('purge_cache');
            log(msg, 'success');
            loadCache();
        } catch (err) {
            log(`Failed to purge cache: ${err}`, 'error');
        }
    }

    // Load Playbooks
    async function loadPlaybooks() {
        try {
            log('Listing active playbooks from daemon...', 'command');
            const playbooks = await invoke('list_active_playbooks');
            
            if (playbooks.length === 0) {
                playbooksList.innerHTML = `
                    <div class="empty-state">
                        <span class="empty-icon">⚙️</span>
                        <p>No active playbooks found running on the daemon.</p>
                    </div>
                `;
            } else {
                playbooksList.innerHTML = playbooks.map(pb => {
                    const capabilitiesHtml = pb.active_capabilities && pb.active_capabilities.length > 0
                        ? pb.active_capabilities.map(cap => `<span class="cap-pill">${cap.package}@${cap.version}</span>`).join('')
                        : '<span class="text-muted" style="font-size:11px;">None</span>';
                        
                    return `
                        <div class="playbook-card">
                            <div class="playbook-card-header">
                                <h3>${pb.name}</h3>
                                <span class="badge badge-online">Running</span>
                            </div>
                            <div class="playbook-details">
                                <div class="detail-item">
                                    <span class="lbl">Config Path</span>
                                    <span class="val code-text">${pb.config_path}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="lbl">Socket</span>
                                    <span class="val code-text">${pb.socket_path || 'None'}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="lbl">Active Capabilities</span>
                                    <div class="capability-pills">${capabilitiesHtml}</div>
                                </div>
                            </div>
                            <div class="playbook-actions">
                                <button class="btn btn-secondary call-pb-btn" data-name="${pb.name}">📞 Call</button>
                                <button class="btn btn-danger stop-pb-btn" data-name="${pb.name}">⏹️ Stop</button>
                            </div>
                        </div>
                    `;
                }).join('');
                
                // Add event listeners for buttons dynamically
                document.querySelectorAll('.stop-pb-btn').forEach(btn => {
                    btn.addEventListener('click', async () => {
                        const name = btn.getAttribute('data-name');
                        await stopPlaybook(name);
                    });
                });
                
                document.querySelectorAll('.call-pb-btn').forEach(btn => {
                    btn.addEventListener('click', () => {
                        const name = btn.getAttribute('data-name');
                        openCallPlaybookModal(name);
                    });
                });
            }
            log(`Retrieved ${playbooks.length} active playbook workers.`, 'success');
        } catch (err) {
            log(`Failed to list playbooks: ${err}`, 'error');
        }
    }

    // Stop Playbook
    async function stopPlaybook(name) {
        if (!confirm(`Are you sure you want to stop playbook worker "${name}"?`)) {
            return;
        }
        try {
            log(`Requesting stop for playbook "${name}"...`, 'command');
            const msg = await invoke('stop_playbook', { name });
            log(msg, 'success');
            // Clean up by requesting delete database entry or just reload
            await invoke('delete_playbook', { name });
            log(`Deleted playbook worker state mapping for "${name}"`, 'system');
            loadPlaybooks();
            queryDaemonStatus();
        } catch (err) {
            log(`Failed to stop playbook: ${err}`, 'error');
        }
    }

    // Open Call Playbook modal
    function openCallPlaybookModal(name) {
        callPlaybookTargetName.innerText = name;
        callPlaybookNameInput.value = name;
        callResultContainer.classList.add('hidden');
        callPlaybookModal.classList.add('active');
        callPlaybookPayloadInput.focus();
    }

    // Call Playbook form submission
    callPlaybookForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const name = callPlaybookNameInput.value;
        const payloadStr = callPlaybookPayloadInput.value;
        
        let payload;
        try {
            payload = JSON.parse(payloadStr);
        } catch (jsonErr) {
            alert('Invalid JSON payload: ' + jsonErr.message);
            return;
        }
        
        const submitBtn = document.getElementById('submit-call-btn');
        submitBtn.disabled = true;
        submitBtn.innerText = 'Calling...';
        
        try {
            log(`Calling playbook "${name}" with payload...`, 'command');
            const res = await invoke('call_playbook', { name, payload });
            
            callResultBox.innerText = JSON.stringify(res, null, 2);
            callResultContainer.classList.remove('hidden');
            log(`Playbook "${name}" call succeeded.`, 'success');
        } catch (err) {
            callResultBox.innerText = `Error: ${err}`;
            callResultContainer.classList.remove('hidden');
            log(`Playbook "${name}" call failed: ${err}`, 'error');
        } finally {
            submitBtn.disabled = false;
            submitBtn.innerText = 'Send Call';
        }
    });

    // Start Playbook form submission
    startPlaybookForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        
        const name = document.getElementById('playbook-name').value;
        const configPath = document.getElementById('playbook-config-path').value;
        const socketPath = document.getElementById('playbook-socket').value || null;
        const inputDir = document.getElementById('playbook-input-dir').value || null;
        const outputDir = document.getElementById('playbook-output-dir').value || null;
        
        try {
            log(`Launching playbook "${name}" from config "${configPath}"...`, 'command');
            const msg = await invoke('start_playbook', {
                name,
                configPath,
                playbookSocket: socketPath,
                inputDir,
                outputDir
            });
            
            log(msg, 'success');
            startPlaybookModal.classList.remove('active');
            loadPlaybooks();
            queryDaemonStatus();
        } catch (err) {
            log(`Failed to launch playbook: ${err}`, 'error');
            alert(`Failed to launch playbook: ${err}`);
        }
    });

    // Wire global control actions
    quickRefreshBtn.addEventListener('click', queryDaemonStatus);
    refreshAllBtn.addEventListener('click', () => {
        queryDaemonStatus();
        const activeTab = document.querySelector('.nav-btn.active').getAttribute('data-tab');
        if (activeTab === 'cache') loadCache();
        if (activeTab === 'playbooks') loadPlaybooks();
    });
    quickPurgeBtn.addEventListener('click', purgeCache);
    purgeCacheBtn.addEventListener('click', purgeCache);

    // Initial Status Check
    queryDaemonStatus();
    
    // Periodically poll daemon status every 10 seconds
    setInterval(queryDaemonStatus, 10000);
});
