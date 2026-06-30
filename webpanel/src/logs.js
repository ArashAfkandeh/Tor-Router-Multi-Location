import { apiCall } from './api.js';

export const logsService = {
    async fetchLogs() {
        return await apiCall('/logs');
    }
};
