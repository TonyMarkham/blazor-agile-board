// Thin bridge to Tauri APIs (desktop mode only)
window.DesktopInterop = {
    /**
     * Check if running in desktop mode.
     */
    isDesktop: function() {
        return window.PM_CONFIG?.isDesktop === true;
    },

    /**
     * Get server status via Tauri IPC.
     * Returns: { state, websocket_url, health, error }
     */
    getServerStatus: async function() {
        if (!window.__TAURI__) {
            throw new Error('Not in desktop mode');
        }
        return await window.__TAURI__.core.invoke('get_server_status');
    },

    /**
     * Listen for server state changes.
     */
    onServerStateChanged: function(callback) {
        if (!window.__TAURI__) return;
        window.__TAURI__.event.listen('server-state-changed', (event) => {
            callback(event.payload);
        });
    }
};