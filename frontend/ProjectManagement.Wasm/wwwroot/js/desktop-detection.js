// Desktop mode detection and Tauri IPC helpers
// Runs before Blazor loads to detect Tauri environment

(function() {
    'use strict';

    // Detect Tauri desktop mode
    window.PM_CONFIG = {
        isDesktop: !!(window.__TAURI__),
        serverUrl: null  // Will be set by C# after server discovery
    };

    if (window.PM_CONFIG.isDesktop) {
        console.log('[Desktop Mode] Tauri detected');
    } else {
        console.log('[Web Mode] Running in browser');
    }

    // Used by TauriService.IsDesktopAsync() - avoids eval() which causes
    // TypeLoadException in some Blazor WASM scenarios
    window.checkTauriAvailable = function() {
        return typeof window.__TAURI__ !== 'undefined' &&
            typeof window.__TAURI__.core !== 'undefined';
    };

    // Used by TauriEventSubscription.DisposeAsync() - avoids eval()
    window.unlistenTauri = function(subscriptionId) {
        var unlisteners = window.__PM_UNLISTENERS__;
        if (unlisteners && typeof unlisteners[subscriptionId] === 'function') {
            unlisteners[subscriptionId]();
            delete unlisteners[subscriptionId];
            return true;
        }
        return false;
    };

    // Sets up a Tauri event listener and stores the unlisten function
    // Called from TauriService.SubscribeToServerStateAsync()
    window.setupTauriListener = async function(dotNetHelper, subscriptionId, eventName) {
        console.log('[Tauri Setup] Setting up listener for:', eventName, 'subscriptionId:', subscriptionId);
        var unlisten = await window.__TAURI__.event.listen(
            eventName,
            async function(event) {
                console.log('[Tauri Event] Received:', eventName, 'Payload:', JSON.stringify(event.payload));
                await dotNetHelper.invokeMethodAsync('HandleEventAsync', event.payload);
            }
        );

        window.__PM_UNLISTENERS__ = window.__PM_UNLISTENERS__ || {};
        window.__PM_UNLISTENERS__[subscriptionId] = unlisten;
        console.log('[Tauri Setup] Listener registered successfully');
    };
})();