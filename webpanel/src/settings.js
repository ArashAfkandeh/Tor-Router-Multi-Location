import { apiCall } from './api.js';

export const settingsService = {
    async fetchSettings() {
        return await apiCall('/settings');
    },

    async saveSettings(form) {
        const payload = { ...form };
        payload.web_panel_port = parseInt(payload.web_panel_port, 10);
        if (!payload.admin_password) delete payload.admin_password;
        
        return await apiCall('/settings', { method: 'PUT', body: JSON.stringify(payload) });
    }
};
