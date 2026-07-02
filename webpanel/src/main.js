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
    ws: null,
    
    init() {
        this.applyTheme();
        this.checkAuth();
    },

    connectWs() {
        if (this.ws) return;
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        let basePath = window.location.pathname;
        if (basePath.endsWith('.html') || basePath.endsWith('.htm')) basePath = basePath.substring(0, basePath.lastIndexOf('/'));
        if (basePath.endsWith('/')) basePath = basePath.substring(0, basePath.length - 1);
        
        const wsUrl = `${protocol}//${window.location.host}${basePath}/api/ws`;
        
        this.ws = new WebSocket(wsUrl);
        
        this.ws.onmessage = (event) => {
            try {
                const payload = JSON.parse(event.data);
                if (payload.routes) {
                    const localNodesMap = {};
                    if (this.nodes) {
                        this.nodes.forEach(n => { localNodesMap[n.id] = n; });
                    }
                    
                    this.nodes = payload.routes.map(serverNode => {
                        const localNode = localNodesMap[serverNode.id];
                        if (localNode && localNode._frontendProbed) {
                            serverNode.latency = localNode.latency;
                            serverNode.status = localNode.status;
                            if (localNode.tor_ip) serverNode.tor_ip = localNode.tor_ip;
                            serverNode._frontendProbed = true;
                        }
                        return serverNode;
                    });
                    
                    this.metrics.total = this.nodes.length;
                    this.metrics.healthy = this.nodes.filter(n => n.status === 'healthy').length;
                    this.metrics.error = this.nodes.filter(n => n.status === 'error').length;
                }
                if (payload.logs) {
                    const logsArray = Array.isArray(payload.logs) ? payload.logs : payload.logs.split('\n');
                    const filtered = logsArray.filter(l => l && l.trim().length > 0);
                    // Emit a custom event so the logs modal can update itself without polling
                    window.dispatchEvent(new CustomEvent('logs-updated', { detail: filtered }));
                }
            } catch (e) {
                console.error('Failed to parse WS message', e);
            }
        };

        this.ws.onclose = () => {
            this.ws = null;
            if (this.isAuthenticated) {
                setTimeout(() => this.connectWs(), 3000);
            }
        };
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
            this.connectWs();
            this.startPeriodicFetch();
        }
    },
    
    async login(username, password, errorCallback) {
        const res = await auth.login(username, password);
        if (res.error) {
            errorCallback(res.error);
        } else {
            this.isAuthenticated = true;
            this.fetchNodes();
            this.connectWs();
            this.startPeriodicFetch();
        }
    },
    
    logout() {
        auth.logout();
        this.isAuthenticated = false;
        this.nodes = [];
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        if (this.fetchInterval) {
            clearInterval(this.fetchInterval);
            this.fetchInterval = null;
        }
    },
    
    startPeriodicFetch() {
        if (this.fetchInterval) clearInterval(this.fetchInterval);
        this.fetchInterval = setInterval(() => {
            if (this.isAuthenticated) {
                this.fetchNodes();
            }
        }, 15000); // 15 seconds
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
