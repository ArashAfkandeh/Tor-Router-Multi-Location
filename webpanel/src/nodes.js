import { apiCall } from './api.js';

export const nodesService = {
    async fetchNodes() {
        const res = await apiCall('/routes');
        if (res.error || !res.data) return { error: res.error || 'Unknown error' };
        
        const data = res.data;
        await Promise.all(data.map(async (node) => {
            try {
                const probe = await apiCall(`/routes/${node.id}/probe`);
                if (!probe.error && probe.data && probe.data.latency) {
                    node.latency = probe.data.latency;
                    if (probe.data.tor_ip) node.tor_ip = probe.data.tor_ip;
                }
            } catch (e) {
            }
        }));
        
        data.forEach(node => {
            let latencyNum = null;
            if (node.latency) {
                 const m = String(node.latency).match(/(\d+)/);
                 if (m) latencyNum = parseInt(m[1], 10);
            }
            
            if (latencyNum !== null) {
                node.status = latencyNum < 800 ? 'healthy' : 'warning';
            }
            
            if (node.status === 1 || node.status === 'healthy') node.status = 'healthy';
            else if (node.status === 2 || node.status === 'warning') node.status = 'warning';
            else if (node.status === 0 || node.status === 'error') node.status = 'error';
            
            node._latencyNum = latencyNum;
        });
        
        return { data };
    },
    
    async saveNode(form) {
        const method = form.id ? 'PUT' : 'POST';
        const endpoint = form.id ? `/routes/${form.id}` : '/routes';
        
        const payload = { ...form };
        payload.input_port = parseInt(payload.input_port, 10);
        payload.test_interval_minutes = parseInt(payload.test_interval_minutes, 10) || 10;
        payload.swap_interval_minutes = parseInt(payload.swap_interval_minutes, 10) || 60;

        return await apiCall(endpoint, { method, body: JSON.stringify(payload) });
    },
    
    async deleteNode(id) {
        return await apiCall(`/routes/${id}`, { method: 'DELETE' });
    },
    
    async restartNode(id) {
        return await apiCall(`/routes/${id}/restart`, { method: 'POST' });
    }
};
