const AUTH_URL = window.location.origin;
const TOKEN_EXPIRY_LEEWAY_SECONDS = 30;

function clearToken() {
    localStorage.removeItem('token');
}

function decodeJwtPayload(token) {
    const payload = token.split('.')[1];
    if (!payload) return null;

    const base64 = payload.replace(/-/g, '+').replace(/_/g, '/');
    const padded = base64.padEnd(base64.length + (4 - base64.length % 4) % 4, '=');
    return JSON.parse(atob(padded));
}

function isExpired(token) {
    try {
        const payload = decodeJwtPayload(token);
        if (!payload?.exp) return true;

        const now = Math.floor(Date.now() / 1000);
        return payload.exp <= now + TOKEN_EXPIRY_LEEWAY_SECONDS;
    } catch {
        return true;
    }
}

export function login() {
    window.location.href = `${AUTH_URL}/auth/google/login`;
}

export function logout() {
    clearToken();
    window.location.href = '/';
}

export function reLogin() {
    clearToken();
    login();
}

export async function handleCallback() {
    const hash = new URLSearchParams(window.location.hash.replace(/^#/, ''));
    const token = hash.get('token');

    if (token) {
        localStorage.setItem('token', token);
        window.history.replaceState({}, '', '/');
        return token;
    }

    const params = new URLSearchParams(window.location.search);
    const authError = params.get('auth_error');
    if (authError) {
        clearToken();
        window.history.replaceState({}, '', '/');
        throw new Error(authError);
    }

    return null;
}

export function getToken() {
    const token = localStorage.getItem('token');
    if (!token) return null;

    if (isExpired(token)) {
        clearToken();
        return null;
    }

    return token;
}
