import Alpine from 'alpinejs';
import { apiCall } from './api.js';
import { auth } from './auth.js';
import { t } from './i18n.js';
import { nodesService } from './nodes.js';
import { settingsService } from './settings.js';
import { logsService } from './logs.js';
import { polyfillCountryFlagEmojis } from "country-flag-emoji-polyfill";

polyfillCountryFlagEmojis();

window.Alpine = Alpine;
window.alpineApiCall = apiCall;
window.nodesService = nodesService;
window.settingsService = settingsService;
window.logsService = logsService;

Alpine.store('app', {
    lang: localStorage.getItem('lang') || 'en',
    theme: localStorage.getItem('theme') || 'dark',
    isAuthenticated: false,
    nodes: [],
    metrics: { total: 0, healthy: 0, error: 0 },
    
    init() {
        this.applyTheme();
        this.checkAuth();
        
        setInterval(() => {
            if (this.isAuthenticated && !document.hidden) {
                this.fetchNodes();
            }
        }, 10000);
        
        document.addEventListener('visibilitychange', () => {
            if (!document.hidden && this.isAuthenticated) {
                this.fetchNodes();
            }
        });
    },
    
    t(key) {
        return t(key, this.lang);
    },
    
    toggleLang() {
        this.lang = this.lang === 'en' ? 'fa' : 'en';
        localStorage.setItem('lang', this.lang);
        document.body.setAttribute('dir', this.lang === 'fa' ? 'rtl' : 'ltr');
    },
    
    toggleTheme() {
        this.theme = this.theme === 'dark' ? 'light' : 'dark';
        localStorage.setItem('theme', this.theme);
        this.applyTheme();
    },
    
    applyTheme() {
        if (this.theme === 'dark') {
            document.documentElement.classList.add('dark');
        } else {
            document.documentElement.classList.remove('dark');
        }
    },
    
    async checkAuth() {
        const isAuth = await auth.check();
        this.isAuthenticated = isAuth;
        if (isAuth) {
            this.fetchNodes();
        }
    },
    
    async login(username, password, errorCallback) {
        const res = await auth.login(username, password);
        if (res.error) {
            errorCallback(res.error);
        } else {
            this.isAuthenticated = true;
            this.fetchNodes();
        }
    },
    
    logout() {
        auth.logout();
        this.isAuthenticated = false;
        this.nodes = [];
    },
    
    async fetchNodes() {
        const res = await nodesService.fetchNodes();
        if (!res.error && res.data) {
            const data = res.data;
            this.nodes = data;
            this.metrics.total = data.length;
            this.metrics.healthy = data.filter(n => n.status === 'healthy').length;
            this.metrics.error = data.filter(n => n.status === 'error').length;
        }
    },

    getCountryFlagEmoji(countryCode) {
        if (!countryCode) return '';
        const match = String(countryCode).trim().toUpperCase().match(/[A-Z]{2}/);
        if (!match) return '';
        return match[0].split('').map(char => String.fromCodePoint(0x1F1E6 + char.charCodeAt(0) - 65)).join('');
    },

    formatTime(ts) {
        if (!ts) return '-';
        return new Date(Number(ts)).toLocaleTimeString([], {hour12:false, hour:'2-digit', minute:'2-digit', second:'2-digit'});
    },

    async restartNode(id) {
        Alpine.store('toast').show(`Restarting...`, 'info');
        const res = await nodesService.restartNode(id);
        if (res.error) Alpine.store('toast').show(`Failed to restart: ${res.error}`, 'error');
        else {
            Alpine.store('toast').show(`Node restarted.`, 'success');
            this.fetchNodes();
        }
    },

    async deleteNode(id) {
        const res = await nodesService.deleteNode(id);
        if (res.error) {
            Alpine.store('toast').show(`Failed to delete: ${res.error}`, 'error');
            return false;
        } else {
            Alpine.store('toast').show(`Node deleted.`, 'success');
            this.fetchNodes();
            return true;
        }
    }
});

Alpine.store('toast', {
    toasts: [],
    show(message, type = 'info') {
        const id = Date.now();
        this.toasts.push({ id, message, type });
        setTimeout(() => {
            this.toasts = this.toasts.filter(t => t.id !== id);
        }, 3000);
    }
});

Alpine.start();
