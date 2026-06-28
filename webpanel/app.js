// Extract base path dynamically from current URL so API calls work correctly
// even if panel is hosted under a subdirectory (e.g. /secret-panel/).
const pathSegments = window.location.pathname.replace(/\/$/, '').split('/');
// if the last segment is e.g. "index.html", we should ideally remove it, 
// but since the server routes SPA appropriately, typically pathname is just the base path.
// Actually, the simplest relative way is just use a relative string, or compute absolute.
let basePath = window.location.pathname;
if (basePath.endsWith('.html') || basePath.endsWith('.htm')) {
    basePath = basePath.substring(0, basePath.lastIndexOf('/'));
}
if (!basePath.endsWith('/')) {
    basePath += '/';
}
basePath = basePath.replace(/\/$/, '');
const API_BASE = basePath + '/api';

// --- i18n ---
const translations = {
    en: {
        login_title: "ToRouter Control",
        login_desc: "Sign in to manage your private nodes.",
        login_user: "Username",
        login_pass: "Password",
        login_btn: "Sign In",
        nav_title: "ToRouter",
        nav_logout: "Logout",
        dash_title: "Exit Nodes",
        dash_desc: "Manage and monitor active Tor routing nodes.",
        btn_restart_all: "Restart All",
        btn_add_node: "Add Node",
        btn_countries: "Tor Countries",
        modal_route_title: "Create Node",
        modal_route_name: "Name",
        modal_route_bind: "Bind Address",
        modal_route_port: "Input Port",
        modal_route_country: "Country Code",
        modal_route_interval: "Test Interval (Min)",
        modal_route_swap: "Swap (Min)",
        modal_route_user: "Username (Opt)",
        modal_route_pass: "Password (Opt)",
        modal_btn_cancel: "Cancel",
        modal_route_btn_save: "Save Node",
        modal_del_title: "Delete Node",
        modal_del_msg: "Are you sure you want to delete '{name}'? This action cannot be undone.",
        modal_del_btn_confirm: "Delete",
        modal_set_title: "Settings",
        modal_set_net: "Network",
        modal_set_web_bind: "Web Panel Bind",
        modal_set_web_port: "Web Port",
        modal_set_api_port: "API Port",
        modal_set_port: "Port",
        modal_set_auth: "Admin Credentials",
        modal_set_user: "Admin Username",
        modal_set_pass: "New Admin Password",
        modal_set_btn_save: "Update Settings",
        modal_set_ssl: "Auto-SSL (Let's Encrypt)",
        modal_set_domain: "Domain / Subdomain",
        modal_set_email: "ACME Email (Optional)",
        modal_set_ssl_help: "If domain is set, panel will automatically fetch and renew SSL certs. Panel MUST be accessible via port 443 externally.",
        nodes_empty_title: "No nodes configured",
        nodes_empty_desc: "Create a new routing node to get started.",
        metric_healthy: "Healthy",
        metric_warning: "Warning",
        metric_error: "Error",
        card_port: "Input Port",
        card_ip: "Tor Exit IP",
        card_latency: "Latency",
        card_auth: "Auth req",
        card_yes: "Yes",
        card_no: "No",
        card_restart: "Restart",
        btn_view_logs: "Logs",
        modal_logs_title: "Application Logs",
        log_level_all: "All Levels",
        log_level_info: "INFO",
        log_level_warn: "WARN",
        log_level_error: "ERROR",
        log_level_debug: "DEBUG",
        copied_to_clipboard: "Logs copied to clipboard"
    },
    fa: {
        login_title: "کنترل پنل تور روتر",
        login_desc: "برای مدیریت نودهای خصوصی وارد شوید.",
        login_user: "نام کاربری",
        login_pass: "رمز عبور",
        login_btn: "ورود",
        nav_title: "تور روتر",
        nav_logout: "خروج",
        dash_title: "نودهای خروجی",
        dash_desc: "نودهای مسیریابی تور را مدیریت و مشاهده کنید.",
        btn_restart_all: "راه‌اندازی مجدد همه",
        btn_add_node: "افزودن نود",
        btn_countries: "کشورهای Tor",
        modal_route_title: "ایجاد نود",
        modal_route_name: "نام",
        modal_route_bind: "آدرس اتصال",
        modal_route_port: "پورت ورودی",
        modal_route_country: "کد کشور",
        modal_route_interval: "فاصله زمانی تست (دقیقه)",
        modal_route_swap: "تعویض IP (دقیقه)",
        modal_route_user: "نام کاربری (اختیاری)",
        modal_route_pass: "رمز عبور (اختیاری)",
        modal_btn_cancel: "انصراف",
        modal_route_btn_save: "ذخیره نود",
        modal_del_title: "حذف نود",
        modal_del_msg: "آیا مطمئن هستید که می‌خواهید نود '{name}' را حذف کنید؟ این عمل قابل بازگشت نیست.",
        modal_del_btn_confirm: "حذف",
        modal_set_title: "تنظیمات",
        modal_set_net: "شبکه",
        modal_set_web_bind: "آدرس اتصال رابط وب",
        modal_set_web_port: "پورت رابط وب",
        modal_set_api_port: "پورت API",
        modal_set_port: "پورت",
        modal_set_auth: "اطلاعات مدیر",
        modal_set_user: "نام کاربری مدیر",
        modal_set_pass: "رمز عبور جدید مدیر",
        modal_set_btn_save: "بروزرسانی تنظیمات",
        modal_set_ssl: "دریافت خودکار SSL",
        modal_set_domain: "دامنه / ساب‌دامنه",
        modal_set_email: "ایمیل (اختیاری)",
        modal_set_ssl_help: "در صورت تنظیم دامنه، پنل به صورت خودکار گواهینامه SSL را دریافت و تمدید می‌کند. توجه کنید که برای این کار پورت پنل باید روی 443 از بیرون در دسترس باشد.",
        nodes_empty_title: "هیچ نودی تنظیم نشده است",
        nodes_empty_desc: "برای شروع یک نود جدید ایجاد کنید.",
        metric_healthy: "سالم",
        metric_warning: "هشدار",
        metric_error: "خطا",
        card_port: "پورت ورودی",
        card_ip: "آی‌پی خروجی",
        card_latency: "تاخیر",
        card_auth: "نیاز به احراز",
        card_yes: "بله",
        card_no: "خیر",
        card_restart: "راه‌اندازی مجدد",
        btn_view_logs: "لاگ‌ها",
        modal_logs_title: "لاگ‌های سیستم",
        log_level_all: "همه سطوح",
        log_level_info: "اطلاعات (INFO)",
        log_level_warn: "هشدار (WARN)",
        log_level_error: "خطا (ERROR)",
        log_level_debug: "دیباگ (DEBUG)",
        copied_to_clipboard: "لاگ‌ها در حافظه کپی شدند"
    }
};

// Add last_check translation keys
translations.en.last_check = "Last Check";
translations.fa.last_check = "آخرین بررسی";

let currentLang = localStorage.getItem('lang') || 'en';

// --- State Management ---
let sessionState = {
    isAuthenticated: false,
    pollInterval: null
};

let nodesData = [];
let deleteTargetId = null;
let deleteTargetName = null;

// --- DOM Elements ---
const el = {
    viewLogin: document.getElementById('view-login'),
    viewDashboard: document.getElementById('view-dashboard'),
    formLogin: document.getElementById('form-login'),
    loginError: document.getElementById('login-error'),
    
    // Dashboard
    nodesContainer: document.getElementById('nodes-container'),
    metricsContainer: document.getElementById('metrics-container'),
    btnRefresh: document.getElementById('btn-refresh'),
    btnLogout: document.getElementById('btn-logout'),
    btnCreateRoute: document.getElementById('btn-create-route'),
    btnSettings: document.getElementById('btn-settings'),
    btnTheme: document.getElementById('btn-theme'),
    btnLang: document.getElementById('btn-lang'),
    btnThemeLogin: document.getElementById('btn-theme-login'),
    btnLangLogin: document.getElementById('btn-lang-login'),
    
    // Route Modal
    modalRoute: document.getElementById('modal-route'),
    formRoute: document.getElementById('form-route'),
    routeTitle: document.getElementById('modal-route-title'),
    routeInputs: {
        id: document.getElementById('route-id'),
        name: document.getElementById('route-name'),
        bind: document.getElementById('route-bind'),
        port: document.getElementById('route-port'),
        country: document.getElementById('route-country'),
        interval: document.getElementById('route-interval'),
        swap: document.getElementById('route-swap'),
        user: document.getElementById('route-user'),
        pass: document.getElementById('route-pass')
    },
    modalRouteCloses: document.querySelectorAll('.modal-close'),

    // Settings Modal
    modalSettings: document.getElementById('modal-settings'),
    
    // Delete Modal
    modalDelete: document.getElementById('modal-delete'),
    deleteMsg: document.getElementById('delete-node-msg'),
    btnConfirmDelete: document.getElementById('modal-delete-btn-confirm'),
    closeDeleteBtns: document.querySelectorAll('.modal-close-delete'),
    formSettings: document.getElementById('form-settings'),
    settingsMsg: document.getElementById('settings-msg'),
    settingsInputs: {
        webBind: document.getElementById('set-web-bind'),
        webPort: document.getElementById('set-web-port'),
        adminUser: document.getElementById('set-admin-user'),
        adminPass: document.getElementById('set-admin-pass'),
        webBase: document.getElementById('set-web-base'),
        domain: document.getElementById('set-domain'),
        customCertToggle: document.getElementById('set-custom-cert-toggle'),
        certPath: document.getElementById('set-cert-path'),
        keyPath: document.getElementById('set-key-path'),
        btnGenerateBase: document.getElementById('btn-generate-base'),
        autoSslFields: document.getElementById('auto-ssl-fields'),
        customSslFields: document.getElementById('custom-ssl-fields')
    },
    modalSettingsCloses: document.querySelectorAll('.modal-close-settings'),
    
    // Logs Modal
    modalLogs: document.getElementById('modal-logs'),
    btnCloseLogs: document.getElementById('modal-close-logs'),
    btnRefreshLogs: document.getElementById('btn-refresh-logs'),
    btnViewLogs: document.getElementById('btn-view-logs'),
    btnCopyLogs: document.getElementById('btn-copy-logs'),
    logLevelFilter: document.getElementById('log-level-filter'),
    logsContainer: document.getElementById('logs-container'),

    toastContainer: document.getElementById('toast-container')
};

// Also attach login-modal buttons immediately in case DOMContentLoaded timing differs
el.btnThemeLogin?.addEventListener('click', toggleTheme);
el.btnLangLogin?.addEventListener('click', toggleLang);

// --- Initialization ---
document.addEventListener('DOMContentLoaded', () => {
    console.debug('[App] DOMContentLoaded triggered.');
    initTheme();
    initLang();

    // Check initial auth state by trying to fetch routes
    console.debug('[App] Checking auth status...');
    checkAuthStatus();

    // Attach Event Listeners
    console.debug('[App] Attaching event listeners...');
    el.formLogin.addEventListener('submit', handleLogin);
    el.btnLogout.addEventListener('click', handleLogout);
    el.btnRefresh.addEventListener('click', fetchRoutes);
    el.btnTheme.addEventListener('click', toggleTheme);
    el.btnLang.addEventListener('click', toggleLang);
    el.btnThemeLogin?.addEventListener('click', toggleTheme);
    el.btnLangLogin?.addEventListener('click', toggleLang);
    
    // Route Modal
    el.btnCreateRoute.addEventListener('click', () => openRouteModal());
    el.formRoute.addEventListener('submit', handleRouteSave);
    el.modalRouteCloses.forEach(btn => btn.addEventListener('click', closeRouteModal));

    // Settings Modal
    el.btnSettings.addEventListener('click', openSettingsModal);
    el.formSettings.addEventListener('submit', handleSettingsSave);
    el.modalSettingsCloses.forEach(btn => btn.addEventListener('click', closeSettingsModal));

    // Delete Modal
    el.closeDeleteBtns.forEach(btn => btn.addEventListener('click', closeDeleteModal));
    el.btnConfirmDelete.addEventListener('click', confirmDeleteRoute);

    // Logs Modal
    if (el.btnViewLogs) el.btnViewLogs.addEventListener('click', openLogsModal);
    if (el.btnCloseLogs) el.btnCloseLogs.addEventListener('click', closeLogsModal);
    if (el.btnRefreshLogs) el.btnRefreshLogs.addEventListener('click', fetchLogs);
    if (el.btnCopyLogs) {
        el.btnCopyLogs.addEventListener('click', async () => {
            try {
                await navigator.clipboard.writeText(el.logsContainer.textContent);
                showToast(t('copied_to_clipboard') || 'Logs copied to clipboard', 'success');
            } catch (err) {
                showToast('Failed to copy logs', 'error');
            }
        });
    }
    if (el.logLevelFilter) {
        el.logLevelFilter.addEventListener('change', () => {
            renderFilteredLogs();
        });
    }

    // Visibility
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            stopPoll();
        } else if (sessionState.isAuthenticated) {
            fetchRoutes();
            startPoll();
        }
    });
});

// --- Theme ---
function initTheme() {
    const stored = localStorage.getItem('theme');
    if (stored) {
        if (stored === 'dark') document.documentElement.classList.add('dark');
        else document.documentElement.classList.remove('dark');
    } else {
        // Default to dark when no preference is stored
        document.documentElement.classList.add('dark');
    }
}

function toggleTheme() {
    const isDark = document.documentElement.classList.toggle('dark');
    localStorage.setItem('theme', isDark ? 'dark' : 'light');
}

// --- i18n Functions ---
function initLang() {
    const langLabel = currentLang === 'en' ? 'FA' : 'EN';
    el.btnLang.textContent = langLabel;
    if (el.btnLangLogin) el.btnLangLogin.textContent = langLabel;
    if (currentLang === 'fa') {
        document.body.setAttribute('dir', 'rtl');
    } else {
        document.body.setAttribute('dir', 'ltr');
    }
    applyTranslations();
}

function toggleLang() {
    currentLang = currentLang === 'en' ? 'fa' : 'en';
    localStorage.setItem('lang', currentLang);
    initLang();
    // Force re-render of nodes and metrics to apply new translations
    if (nodesData.length) {
        el.nodesContainer.innerHTML = ''; // clears currentIds
        renderDashboard(nodesData);
    }
}

function t(key) {
    return translations[currentLang][key] || key;
}

function applyTranslations() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (translations[currentLang][key]) {
            if (el.tagName === 'INPUT' && el.type === 'button') {
                el.value = translations[currentLang][key];
            } else {
                el.textContent = translations[currentLang][key];
            }
        }
    });
}

// --- API Helpers ---
async function apiCall(endpoint, options = {}) {
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
            handleLogout(); // Session expired or invalid
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

// --- Auth flows ---
async function checkAuthStatus() {
    console.debug('[App] Fetching routes to check auth status...');
    // If we can get routes, we are authenticated
    const res = await apiCall('/routes');
    if (res.error && res.status === 401) {
        console.debug('[App] Not authenticated (401). Showing login screen.');
        showLogin();
    } else {
        console.debug('[App] Authenticated successfully. Showing dashboard.');
        if (res.data) {
            nodesData = res.data;
        }
        showDashboard();
        if (res.data) {
            console.debug('[App] Rendering dashboard with data:', nodesData);
            renderDashboard(nodesData);
            // Optionally run fetchRoutes to populate latency right away
            fetchRoutes();
        }
    }
}

async function handleLogin(e) {
    e.preventDefault();
    const username = document.getElementById('login-username').value;
    const password = document.getElementById('login-password').value;
    
    el.loginError.classList.add('hidden');
    el.formLogin.querySelector('button').disabled = true;

    const res = await apiCall('/login', {
        method: 'POST',
        body: JSON.stringify({ username, password })
    });

    el.formLogin.querySelector('button').disabled = false;

    if (res.error) {
        el.loginError.textContent = res.error;
        el.loginError.classList.remove('hidden');
    } else {
        el.formLogin.reset();
        showDashboard();
        fetchRoutes(); // Initial fetch
    }
}

function handleLogout() {
    sessionState.isAuthenticated = false;
    stopPoll();
    // Clear cookies generic way
    document.cookie = "session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;";
    showLogin();
}

function showLogin() {
    el.viewDashboard.classList.add('hidden');
    el.viewDashboard.classList.remove('flex');
    el.viewLogin.classList.remove('hidden');
}

function showDashboard() {
    el.viewLogin.classList.add('hidden');
    el.viewDashboard.classList.remove('hidden');
    el.viewDashboard.classList.add('flex');
    sessionState.isAuthenticated = true;
    startPoll();
}

// --- Polling & Data Fetching ---
function startPoll() {
    if (sessionState.pollInterval) clearInterval(sessionState.pollInterval);
    sessionState.pollInterval = setInterval(fetchRoutes, 10000); // Poll every 10 seconds
}

function stopPoll() {
    if (sessionState.pollInterval) clearInterval(sessionState.pollInterval);
}

async function fetchRoutes() {
    if (!sessionState.isAuthenticated) return;
    
    // Add spinning animation to refresh icon safely
    const icon = el.btnRefresh.querySelector('svg');
    if (icon) icon.classList.add('animate-spin');

    const res = await apiCall('/routes');
    
    if (icon) icon.classList.remove('animate-spin');

    if (!res.error && res.data) {
        nodesData = res.data;

        // Probe each route for a live latency measurement using the backend probe endpoint.
        // This uses the shared backend probe logic (measure_latency) instead of relying
        // solely on the persisted value in the DB.
        await Promise.all(nodesData.map(async (node) => {
            try {
                const probe = await apiCall(`/routes/${node.id}/probe`);
                if (!probe.error && probe.data && probe.data.latency) {
                    node.latency = probe.data.latency;
                    if (probe.data.tor_ip) node.tor_ip = probe.data.tor_ip;
                }
            } catch (e) {
                // ignore probe errors and keep existing latency
            }
        }));

        renderDashboard(nodesData);
    }
}

// --- Render Views ---

function renderDashboard(data) {
    if (!data) return;

    // Enhance node data with parsed latency and override status BEFORE rendering
    data.forEach(node => {
        let latencyNum = null;
        if (node.latency) {
             const m = String(node.latency).match(/(\d+)/);
             if (m) latencyNum = parseInt(m[1], 10);
        }
        
        // Override status
        if (latencyNum !== null) {
            if (latencyNum < 800) {
                node.status = 1;
            } else {
                node.status = 2;
            }
        }
        
        // Normalize status
        if (node.status === 1 || node.status === 'healthy') node.status = 'healthy';
        else if (node.status === 2 || node.status === 'warning') node.status = 'warning';
        else if (node.status === 0 || node.status === 'error') node.status = 'error';
        
        node._latencyNum = latencyNum;
    });

    renderMetrics(data);
    renderNodes(data);
}

function renderMetrics(data) {
    const total = data.length;
    const healthy = data.filter(n => n.status === 'healthy').length;
    const error = data.filter(n => n.status === 'error').length;
    
    // total translation not needed, we can just use "Total Nodes" from html directly or add it. Let's add it. Wait, I didn't add it to translations. Let me just use t() where I added.
    const t_total = currentLang === 'en' ? 'Total Nodes' : 'تعداد کل نودها';

    // Partial update to prevent flashing during polling
    if (el.metricsContainer.children.length === 3) {
        el.metricsContainer.children[0].querySelector('span.value').textContent = total;
        el.metricsContainer.children[1].querySelector('span.value').textContent = healthy;
        el.metricsContainer.children[2].querySelector('span.value').textContent = error;
        
        el.metricsContainer.children[0].querySelector('span.label').textContent = t_total;
        el.metricsContainer.children[1].querySelector('span.label').textContent = t('metric_healthy');
        el.metricsContainer.children[2].querySelector('span.label').textContent = t('metric_error');
        return;
    }

    el.metricsContainer.innerHTML = `
        <div class="flex items-center gap-1.5 px-2.5 py-1 bg-slate-100 dark:bg-slate-800 rounded-full border border-slate-200 dark:border-slate-700">
            <div class="w-1.5 h-1.5 rounded-full bg-blue-500"></div>
            <span class="label text-[10px] font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">${t_total}</span>
            <span class="value text-xs font-bold text-slate-900 dark:text-white">${total}</span>
        </div>
        <div class="flex items-center gap-1.5 px-2.5 py-1 bg-slate-100 dark:bg-slate-800 rounded-full border border-slate-200 dark:border-slate-700">
            <div class="w-1.5 h-1.5 rounded-full bg-emerald-500"></div>
            <span class="label text-[10px] font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">${t('metric_healthy')}</span>
            <span class="value text-xs font-bold text-emerald-600 dark:text-emerald-400">${healthy}</span>
        </div>
        <div class="flex items-center gap-1.5 px-2.5 py-1 bg-slate-100 dark:bg-slate-800 rounded-full border border-slate-200 dark:border-slate-700">
            <div class="w-1.5 h-1.5 rounded-full bg-red-500"></div>
            <span class="label text-[10px] font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">${t('metric_error')}</span>
            <span class="value text-xs font-bold text-red-600 dark:text-red-400">${error}</span>
        </div>
    `;
}

function renderNodes(data) {
    if (!data || data.length === 0) {
        el.nodesContainer.innerHTML = `
            <div class="col-span-full py-16 text-center border-2 border-dashed border-slate-200 dark:border-slate-700 rounded-2xl">
                <div class="w-16 h-16 mx-auto bg-slate-100 dark:bg-slate-800 rounded-full flex items-center justify-center mb-4 text-slate-400">
                    <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>
                </div>
                <h3 data-i18n="nodes_empty_title" class="text-lg font-medium text-slate-900 dark:text-white">No nodes configured</h3>
                <p data-i18n="nodes_empty_desc" class="text-slate-500 dark:text-slate-400 mt-1 mb-6">Create a new routing node to get started.</p>
                <button onclick="openRouteModal()" class="px-5 py-2.5 bg-brand-50 text-brand-700 dark:bg-brand-900/30 dark:text-brand-400 font-medium rounded-lg hover:bg-brand-100 dark:hover:bg-brand-900/50 transition-colors inline-flex items-center gap-2">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    <span data-i18n="btn_add_node">Create Node</span>
                </button>
            </div>
        `;
        return;
    }

    const currentIds = Array.from(el.nodesContainer.children).map(c => c.dataset.id).filter(Boolean);
    const newIds = data.map(n => n.id);
    const hasDiff = currentIds.length !== newIds.length || !currentIds.every(id => newIds.includes(id));

    if (hasDiff) {
        el.nodesContainer.innerHTML = data.map(node => createNodeCard(node)).join('');
    } else {
        data.forEach(node => updateNodeCard(node));
    }
}

function getCountryFlagEmoji(countryCode) {
    if (!countryCode) return '';
    const match = String(countryCode).trim().toUpperCase().match(/[A-Z]{2}/);
    if (!match) return '';
    return match[0].split('').map(char => String.fromCodePoint(0x1F1E6 + char.charCodeAt(0) - 65)).join('');
}

function createNodeCard(node) {
    let statusClass = getStatusClass(node.status);
    let pulseClass = getPulseClass(node.status);
    const countryFlag = getCountryFlagEmoji(node.country_code);
    const latencyColor = node.status === 'healthy' ? 'text-emerald-600 dark:text-emerald-400' : (node.status === 'error' ? 'text-red-500' : 'text-yellow-500');

    return `
        <div data-id="${node.id}" class="bg-white dark:bg-slate-800 rounded-2xl border border-slate-200 dark:border-slate-700 shadow-sm overflow-hidden flex flex-col hover:shadow-md transition-shadow animate-fade-in relative">
            <!-- Status Bar Top -->
            <div data-el="top-bar" class="h-1 w-full ${statusClass}"></div>

            <div class="p-4 flex-1 select-text">
                <div class="flex items-start justify-between gap-3 mb-3">
                    <div class="flex items-center gap-2 min-w-0">
                        <div class="relative flex h-3 w-3">
                            <span data-el="pulse" class="relative inline-flex rounded-full h-3 w-3 ${statusClass} ${pulseClass}"></span>
                        </div>
                        <h3 data-el="name" class="font-semibold text-base text-slate-900 dark:text-white truncate" title="${node.name}">${node.name}</h3>
                    </div>
                    <div class="country-flag-emoji inline-flex items-center justify-center transition-all duration-300 text-3xl">
                        ${countryFlag || node.country_code}
                    </div>
                </div>

                <div class="space-y-2 mb-4 text-sm text-slate-600 dark:text-slate-300">
                    <div class="flex justify-between items-center py-1 border-b border-slate-100 dark:border-slate-700/50">
                        <span class="text-slate-500 dark:text-slate-400 font-medium">${t('card_port')}</span>
                        <span data-el="port" class="font-mono text-slate-900 dark:text-white bg-slate-100 dark:bg-slate-700 px-2 pl-2 rounded">${node.input_port}</span>
                    </div>
                    <div class="flex justify-between items-center py-1 border-b border-slate-100 dark:border-slate-700/50">
                        <span class="text-slate-500 dark:text-slate-400 font-medium">${t('card_ip')}</span>
                        <span data-el="ip" class="font-mono ${node.tor_ip ? 'text-slate-900 dark:text-white' : 'text-slate-400 italic'}">${node.tor_ip || 'Acquiring...'}</span>
                    </div>
                    <div class="flex justify-between items-center py-1 border-b border-slate-100 dark:border-slate-700/50">
                        <span class="text-slate-500 dark:text-slate-400 font-medium">${t('card_latency')}</span>
                        <span data-el="latency" class="font-mono ${latencyColor}">${node.latency || 'N/A'}</span>
                    </div>
                    <div class="flex justify-between items-center py-1">
                        <span data-i18n="last_check" class="text-slate-500 dark:text-slate-400 font-medium">${t('last_check')}</span>
                        <span data-el="last-check" class="font-mono text-slate-900 dark:text-white">${node.last_checked_at ? new Date(Number(node.last_checked_at)).toLocaleTimeString([], {hour12:false, hour:'2-digit', minute:'2-digit', second:'2-digit'}) : '-'}</span>
                    </div>
                    <div class="flex justify-between items-center py-1">
                        <span class="text-slate-500 dark:text-slate-400 font-medium">${t('card_auth')}</span>
                        <span data-el="auth-req" class="font-mono text-slate-900 dark:text-white">${(node.username && node.password) ? '<span class="auth-emoji locked">🔒</span>' : '<span class="auth-emoji unlocked">🔓</span>'}</span>
                    </div>
                </div>
            </div>

            <!-- Card Actions -->
            <div class="bg-slate-50 dark:bg-slate-800/80 p-3 border-t border-slate-100 dark:border-slate-700 flex justify-between items-center gap-2">
                <button onclick="handleRestart('${node.id}', '${node.name}')" class="flex-1 px-2.5 py-2 text-xs font-semibold text-slate-700 bg-white border border-slate-200 hover:bg-slate-50 dark:bg-slate-700 dark:border-slate-600 dark:text-slate-200 dark:hover:bg-slate-600 rounded-xl shadow-sm transition-colors flex items-center justify-center gap-2">
                    <svg class="w-4 h-4 text-slate-500 dark:text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
                    ${t('card_restart')}
                </button>
                <button onclick="openRouteModal('${node.id}')" class="p-2 text-slate-500 hover:text-brand-600 hover:bg-brand-50 dark:text-slate-400 dark:hover:text-brand-400 dark:hover:bg-slate-700 rounded-lg transition-colors border border-transparent shadow-sm" title="Edit">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
                </button>
                <button onclick="handleDelete('${node.id}', '${node.name}')" class="p-2 text-slate-500 hover:text-red-600 hover:bg-red-50 dark:text-slate-400 dark:hover:text-red-400 dark:hover:bg-slate-700 rounded-lg transition-colors border border-transparent shadow-sm" title="Delete">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                </button>
            </div>
        </div>
    `;
}

function updateNodeCard(node) {
    const nodeId = String(node.id);
    const card = Array.from(el.nodesContainer.children).find(card => card.dataset.id === nodeId);
    if (!card) return;

    let statusClass = getStatusClass(node.status);
    let pulseClass = getPulseClass(node.status);
    let latencyColor = node.status === 'healthy' ? 'text-emerald-600 dark:text-emerald-400' : (node.status === 'error' ? 'text-red-500' : 'text-yellow-500');

    // Refresh any changing route fields without rebuilding the full node DOM

    const topBar = card.querySelector('[data-el="top-bar"]');
    const newTopBarClass = `h-1 w-full ${statusClass}`;
    if (topBar && topBar.className !== newTopBarClass) {
        topBar.className = newTopBarClass;
    }

    const pulse = card.querySelector('[data-el="pulse"]');
    const newPulseClass = `relative inline-flex rounded-full h-3 w-3 ${statusClass} ${pulseClass}`;
    if (pulse && pulse.className !== newPulseClass) {
        pulse.className = newPulseClass;
    }

    const latencyEl = card.querySelector('[data-el="latency"]');
    if (latencyEl) {
        const latencyText = node.latency || 'N/A';
        if (latencyEl.textContent !== latencyText) {
            latencyEl.textContent = latencyText;
        }
        const latClass = `font-mono ${latencyColor}`;
        if (latencyEl.className !== latClass) {
            latencyEl.className = latClass;
        }
    }

    const ipEl = card.querySelector('[data-el="ip"]');
    if (ipEl) {
        const ipText = node.tor_ip || 'Acquiring...';
        if (ipEl.textContent !== ipText) {
            ipEl.textContent = ipText;
        }
        const ipClass = node.tor_ip ? 'font-mono text-slate-900 dark:text-white' : 'font-mono text-slate-400 italic';
        if (ipEl.className !== ipClass) {
            ipEl.className = ipClass;
        }
    }

    const portEl = card.querySelector('[data-el="port"]');
    if (portEl) {
        const portText = node.input_port != null ? String(node.input_port) : '-';
        if (portEl.textContent !== portText) {
            portEl.textContent = portText;
        }
    }

    const nameEl = card.querySelector('[data-el="name"]');
    const expectedName = node.name;
    if (nameEl && nameEl.textContent !== expectedName) {
        nameEl.textContent = expectedName;
    }

    const lastCheckEl = card.querySelector('[data-el="last-check"]');
    if (lastCheckEl) {
        const lastText = node.last_checked_at ? new Date(Number(node.last_checked_at)).toLocaleTimeString([], {hour12:false, hour:'2-digit', minute:'2-digit', second:'2-digit'}) : '-';
        if (lastCheckEl.textContent !== lastText) lastCheckEl.textContent = lastText;
    }

    const authEl = card.querySelector('[data-el="auth-req"]');
    if (authEl) {
        const authHtml = (node.username && node.password) ? '<span class="auth-emoji locked">🔒</span>' : '<span class="auth-emoji unlocked">🔓</span>';
        if (authEl.innerHTML !== authHtml) authEl.innerHTML = authHtml;
    }
}

function getStatusClass(status) {
    if (status === 'healthy') return 'bg-emerald-500';
    if (status === 'warning') return 'bg-yellow-500';
    return 'bg-red-500';
}

function getPulseClass(status) {
    if (status === 'healthy') return 'status-pulse-healthy';
    if (status === 'warning') return 'status-pulse-warning';
    return 'status-pulse-error';
}


// --- CRUD Actions ---

async function handleRestart(id, name) {
    showToast(`Restarting ${name}...`, 'info');
    const res = await apiCall(`/routes/${id}/restart`, { method: 'POST' });
    if (res.error) showToast(`Failed to restart: ${res.error}`, 'error');
    else {
        showToast(`${name} restarted.`, 'success');
        fetchRoutes(); // rapid refresh
    }
}

async function handleDelete(id, name) {
    deleteTargetId = id;
    deleteTargetName = name;
    el.deleteMsg.innerText = t('modal_del_msg').replace('{name}', name);
    openDeleteModal();
}

function openDeleteModal() {
    el.modalDelete.classList.remove('hidden');
    el.modalDelete.classList.add('flex');
    // slight delay to allow display block to apply before opacity transition
    setTimeout(() => {
        el.modalDelete.classList.remove('opacity-0');
        el.modalDelete.children[0].classList.remove('scale-95');
    }, 10);
}

function closeDeleteModal() {
    el.modalDelete.classList.add('opacity-0');
    el.modalDelete.children[0].classList.add('scale-95');
    setTimeout(() => {
        el.modalDelete.classList.remove('flex');
        el.modalDelete.classList.add('hidden');
        deleteTargetId = null;
        deleteTargetName = null;
    }, 300);
}

async function confirmDeleteRoute() {
    if (!deleteTargetId) return;
    
    // Disable button to prevent double submit
    const originalText = el.btnConfirmDelete.innerText;
    el.btnConfirmDelete.innerText = '...';
    el.btnConfirmDelete.disabled = true;

    const res = await apiCall(`/routes/${deleteTargetId}`, { method: 'DELETE' });
    
    el.btnConfirmDelete.innerText = originalText;
    el.btnConfirmDelete.disabled = false;

    if (res.error) {
        showToast(`Failed to delete: ${res.error}`, 'error');
    } else {
        showToast(`${deleteTargetName} deleted.`, 'success');
        closeDeleteModal();
        fetchRoutes();
    }
}

// --- Route Modal (Create/Edit) ---

function openRouteModal(id = null) {
    el.formRoute.reset();
    el.routeInputs.id.value = '';
    el.routeTitle.textContent = 'Create Node';

        if (id) {
            const nodeIdStr = String(id);
            const node = nodesData.find(n => String(n.id) === nodeIdStr);
            if (node) {
                el.routeTitle.textContent = t('modal_route_edit') || 'Edit Node';
                el.routeInputs.id.value = node.id;
                el.routeInputs.name.value = node.name;
                el.routeInputs.bind.value = node.bind_address !== '127.0.0.0' ? node.bind_address : '';
                el.routeInputs.port.value = node.input_port;
                el.routeInputs.country.value = node.country_code;
                el.routeInputs.interval.value = node.test_interval_minutes || 10;
                el.routeInputs.swap.value = node.swap_interval_minutes || 1440;
                el.routeInputs.user.value = node.username || '';
                el.routeInputs.pass.value = node.password || '';
            }
        }
    
    // Show Modal
    el.modalRoute.classList.remove('hidden');
    el.modalRoute.classList.add('flex');
    // small timeout for animation
    setTimeout(() => {
        el.modalRoute.classList.remove('opacity-0');
        el.modalRoute.querySelector('div').classList.remove('scale-95');
    }, 10);
}

function closeRouteModal() {
    el.modalRoute.classList.add('opacity-0');
    el.modalRoute.querySelector('div').classList.add('scale-95');
    setTimeout(() => {
        el.modalRoute.classList.remove('flex');
        el.modalRoute.classList.add('hidden');
    }, 300);
}

async function handleRouteSave(e) {
    e.preventDefault();
    const id = el.routeInputs.id.value;
    
    const payload = {
        name: el.routeInputs.name.value,
        bind_address: el.routeInputs.bind.value || null,
        input_port: parseInt(el.routeInputs.port.value, 10),
        country_code: el.routeInputs.country.value,
        test_interval_minutes: parseInt(el.routeInputs.interval.value, 10) || 10,
        swap_interval_minutes: parseInt(el.routeInputs.swap.value, 10) || 1440,
        username: el.routeInputs.user.value || null,
        password: el.routeInputs.pass.value || null
    };

    const isEdit = !!id;
    const endpoint = isEdit ? `/routes/${id}` : '/routes';
    const method = isEdit ? 'PUT' : 'POST';

    const res = await apiCall(endpoint, {
        method,
        body: JSON.stringify(payload)
    });

    if (res.error) {
        showToast(res.error, 'error');
    } else {
        showToast(isEdit ? 'Node updated successfully' : 'Node created successfully', 'success');
        closeRouteModal();
        fetchRoutes();
    }
}


// --- Settings Modal ---

async function openSettingsModal() {
    el.settingsMsg.classList.add('hidden');
    
    // Fetch settings
    const res = await apiCall('/settings');
    if (res.error) {
        showToast('Failed to load settings', 'error');
        return;
    }
    
    const data = res.data;
    el.settingsInputs.webBind.value = data.web_bind_address || '';
    el.settingsInputs.webPort.value = data.web_panel_port || '';
    el.settingsInputs.adminUser.value = data.admin_username || '';
    el.settingsInputs.adminPass.value = '';
    el.settingsInputs.webBase.value = data.web_base_path || '';
    el.settingsInputs.domain.value = data.domain || '';
    el.settingsInputs.certPath.value = data.custom_cert_path || '';
    el.settingsInputs.keyPath.value = data.custom_key_path || '';
    
    el.settingsInputs.customCertToggle.checked = !!data.use_custom_cert;
    updateCustomCertUI();

    el.modalSettings.classList.remove('hidden');
    el.modalSettings.classList.add('flex');
    setTimeout(() => {
        el.modalSettings.classList.remove('opacity-0');
        el.modalSettings.querySelector('div').classList.remove('scale-95');
    }, 10);
}

function updateCustomCertUI() {
    if (el.settingsInputs.customCertToggle.checked) {
        el.settingsInputs.domain.disabled = true;
        el.settingsInputs.domain.classList.add('opacity-50', 'cursor-not-allowed');
        el.settingsInputs.customSslFields.classList.remove('hidden');
    } else {
        el.settingsInputs.domain.disabled = false;
        el.settingsInputs.domain.classList.remove('opacity-50', 'cursor-not-allowed');
        el.settingsInputs.customSslFields.classList.add('hidden');
    }
}

el.settingsInputs.customCertToggle.addEventListener('change', updateCustomCertUI);

el.settingsInputs.btnGenerateBase.addEventListener('click', () => {
    const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
    let token = '/';
    for (let i = 0; i < 15; i++) {
        token += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    el.settingsInputs.webBase.value = token;
});

function closeSettingsModal() {
    el.modalSettings.classList.add('opacity-0');
    el.modalSettings.querySelector('div').classList.add('scale-95');
    setTimeout(() => {
        el.modalSettings.classList.remove('flex');
        el.modalSettings.classList.add('hidden');
    }, 300);
}

async function handleSettingsSave(e) {
    e.preventDefault();
    
    const payload = {};
    if (el.settingsInputs.webBind.value) payload.web_bind_address = el.settingsInputs.webBind.value;
    if (el.settingsInputs.webPort.value) {
        const p = parseInt(el.settingsInputs.webPort.value, 10);
        payload.web_panel_port = p;
        payload.api_port = p; // enforce single shared port
    }
    if (el.settingsInputs.adminUser.value) payload.admin_username = el.settingsInputs.adminUser.value;
    if (el.settingsInputs.adminPass.value) payload.admin_password = el.settingsInputs.adminPass.value;
    
    payload.domain = el.settingsInputs.domain.value.trim() || null;
    payload.use_custom_cert = el.settingsInputs.customCertToggle.checked;
    payload.custom_cert_path = el.settingsInputs.certPath.value.trim() || null;
    payload.custom_key_path = el.settingsInputs.keyPath.value.trim() || null;
    
    let base = el.settingsInputs.webBase.value.trim();
    if (base && !base.startsWith('/')) base = '/' + base;
    payload.web_base_path = base || '';

    const res = await apiCall('/settings', {
        method: 'PUT',
        body: JSON.stringify(payload)
    });

    if (res.error) {
        el.settingsMsg.textContent = res.error;
        el.settingsMsg.className = "mt-3 text-sm font-medium text-red-500";
        el.settingsMsg.classList.remove('hidden');
    } else {
        el.settingsMsg.textContent = "Settings saved! Restarting web server...";
        el.settingsMsg.className = "mt-3 text-sm font-medium text-emerald-500";
        el.settingsMsg.classList.remove('hidden');

        const newPort = (res.data && res.data.web_panel_port) ? res.data.web_panel_port : (payload.web_panel_port || el.settingsInputs.webPort.value);

        setTimeout(() => {
            closeSettingsModal();
            try {
                const host = window.location.hostname;
                const proto = window.location.protocol;
                let newPath = payload.web_base_path;
                const target = `${proto}//${host}:${newPort}${newPath}`;
                window.location.href = target;
            } catch (e) {
                location.reload();
            }
        }, 1400);
    }
}


// --- Toasts UI ---
function showToast(message, type = 'info') {
    if (type === 'error') {
        console.error(`[Toast Error] ${message}`);
    } else if (type === 'success') {
        console.log(`[Toast Success] ${message}`);
    } else {
        console.log(`[Toast Info] ${message}`);
    }

    const toast = document.createElement('div');
    
    let colorClasses = 'bg-slate-800 text-white';
    let icon = `<svg class="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>`;
    
    if (type === 'success') {
        icon = `<svg class="w-5 h-5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>`;
    } else if (type === 'error') {
        icon = `<svg class="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>`;
    }

    toast.className = `flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg font-medium text-sm transition-all duration-300 transform translate-y-full opacity-0 ${colorClasses}`;
    toast.innerHTML = `${icon} <span>${message}</span>`;
    
    el.toastContainer.appendChild(toast);
    
    // Animate in
    setTimeout(() => {
        toast.classList.remove('translate-y-full', 'opacity-0');
    }, 10);

    // Animate out
    setTimeout(() => {
        toast.classList.add('opacity-0', 'translate-x-full');
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

// --- Logs Modal ---

function openLogsModal() {
    el.modalLogs.classList.remove('hidden');
    el.modalLogs.classList.add('flex');
    setTimeout(() => {
        el.modalLogs.classList.remove('opacity-0');
        el.modalLogs.querySelector('div').classList.remove('scale-95');
    }, 10);
    fetchLogs();
}

function closeLogsModal() {
    el.modalLogs.classList.add('opacity-0');
    el.modalLogs.querySelector('div').classList.add('scale-95');
    setTimeout(() => {
        el.modalLogs.classList.remove('flex');
        el.modalLogs.classList.add('hidden');
    }, 300);
}

let allLogsData = [];

function renderFilteredLogs() {
    if (!allLogsData || allLogsData.length === 0) {
        el.logsContainer.textContent = "No logs available.";
        return;
    }
    const filter = el.logLevelFilter ? el.logLevelFilter.value.toLowerCase() : 'all';
    
    if (filter === 'all') {
        el.logsContainer.textContent = allLogsData.join("");
    } else {
        const filtered = allLogsData.filter(line => line.toLowerCase().includes(`[${filter}]`));
        el.logsContainer.textContent = filtered.length > 0 ? filtered.join("") : "No logs matching filter.";
    }
    el.logsContainer.parentElement.scrollTop = el.logsContainer.parentElement.scrollHeight;
}

async function fetchLogs() {
    el.logsContainer.textContent = "Loading logs...";
    const res = await apiCall('/logs');
    if (res.error) {
        el.logsContainer.textContent = "Error fetching logs: " + res.error;
    } else {
        allLogsData = res.data.logs || [];
        renderFilteredLogs();
    }
}
