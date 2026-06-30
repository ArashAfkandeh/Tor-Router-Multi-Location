// API Client
let basePath = window.location.pathname;
if (basePath.endsWith('.html') || basePath.endsWith('.htm')) {
    basePath = basePath.substring(0, basePath.lastIndexOf('/'));
}
if (!basePath.endsWith('/')) {
    basePath += '/';
}
basePath = basePath.replace(/\/$/, '');
const API_BASE = basePath + '/api';

export async function apiCall(endpoint, options = {}) {
    console.debug(`[API Request] -> ${options.method || 'GET'} ${API_BASE}${endpoint}`, options.body ? JSON.parse(options.body) : '');
    try {
        const res = await fetch(`${API_BASE}${endpoint}`, {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...(options.headers || {})
            }
        });

        if (res.status === 401) {
            console.warn(`[API Response] 401 Unauthorized <- ${endpoint}`);
            return { error: 'Unauthorized', status: 401 };
        }

        const data = await res.json().catch(() => ({}));
        
        if (!res.ok) {
            console.error(`[API Response] Error ${res.status} <- ${endpoint}:`, data.error || data || `Error ${res.status}`);
            return { error: data.error || data || `Error ${res.status}`, status: res.status };
        }
        
        console.debug(`[API Response] OK 200 <- ${endpoint}`, data);
        return { data, status: res.status };
    } catch (err) {
        console.error(`[API] Network connection failed on ${endpoint}:`, err);
        return { error: 'Network connection failed', status: 0 };
    }
}
