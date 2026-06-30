import { apiCall } from './api.js';

export const auth = {
    isAuthenticated: false,
    
    async check() {
        const res = await apiCall('/routes');
        if (res.error && res.status === 401) {
            this.isAuthenticated = false;
            return false;
        }
        this.isAuthenticated = true;
        return true;
    },

    async login(username, password) {
        const res = await apiCall('/login', {
            method: 'POST',
            body: JSON.stringify({ username, password })
        });
        
        if (!res.error) {
            this.isAuthenticated = true;
        }
        return res;
    },

    logout() {
        this.isAuthenticated = false;
        document.cookie = "session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;";
    }
};
