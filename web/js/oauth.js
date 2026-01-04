/**
 * Google OAuth PKCE flow for static sites
 * Based on: https://github.com/aaronpk/pkce-vanilla-js
 */

const GOOGLE_CLIENT_ID = '1055019721589-7gh63ujmm7fekedmdnquo1f2fh9l5p3g.apps.googleusercontent.com';
const GOOGLE_AUTH_ENDPOINT = 'https://accounts.google.com/o/oauth2/v2/auth';
const GOOGLE_TOKEN_ENDPOINT = 'https://oauth2.googleapis.com/token';
const OAUTH_SCOPE = 'openid email profile';

// PKCE utilities
function generateRandomString(length) {
    const array = new Uint8Array(length);
    crypto.getRandomValues(array);
    return base64URLEncode(array);
}

function base64URLEncode(buffer) {
    return btoa(String.fromCharCode(...buffer))
        .replace(/\+/g, '-')
        .replace(/\//g, '_')
        .replace(/=/g, '');
}

async function sha256(plain) {
    const encoder = new TextEncoder();
    const data = encoder.encode(plain);
    return await crypto.subtle.digest('SHA-256', data);
}

async function generateCodeChallenge(verifier) {
    const hashed = await sha256(verifier);
    return base64URLEncode(new Uint8Array(hashed));
}

// OAuth flow
export async function initiateOAuthLogin() {
    const redirectUri = window.location.origin + window.location.pathname;
    const codeVerifier = generateRandomString(64);
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const state = generateRandomString(16);

    sessionStorage.setItem('pkce_code_verifier', codeVerifier);
    sessionStorage.setItem('oauth_state', state);

    const params = new URLSearchParams({
        client_id: GOOGLE_CLIENT_ID,
        redirect_uri: redirectUri,
        response_type: 'code',
        scope: OAUTH_SCOPE,
        code_challenge: codeChallenge,
        code_challenge_method: 'S256',
        state: state,
    });

    window.location.href = `${GOOGLE_AUTH_ENDPOINT}?${params}`;
}

export async function handleOAuthCallback() {
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');
    const error = urlParams.get('error');
    const state = urlParams.get('state');

    if (error) {
        console.error('[OAuth] Error:', error);
        return null;
    }

    if (!code) {
        return null; // Not an OAuth callback
    }

    const savedState = sessionStorage.getItem('oauth_state');
    if (state !== savedState) {
        console.error('[OAuth] State mismatch - possible CSRF attack');
        return null;
    }

    const codeVerifier = sessionStorage.getItem('pkce_code_verifier');
    if (!codeVerifier) {
        console.error('[OAuth] Code verifier not found');
        return null;
    }

    const redirectUri = window.location.origin + window.location.pathname;

    try {
        const response = await fetch(GOOGLE_TOKEN_ENDPOINT, {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
            body: new URLSearchParams({
                client_id: GOOGLE_CLIENT_ID,
                code: code,
                code_verifier: codeVerifier,
                grant_type: 'authorization_code',
                redirect_uri: redirectUri,
            }),
        });

        if (!response.ok) {
            throw new Error(`Token exchange failed: ${response.statusText}`);
        }

        const tokens = await response.json();

        sessionStorage.removeItem('pkce_code_verifier');
        sessionStorage.removeItem('oauth_state');
        window.history.replaceState({}, document.title, window.location.pathname);

        localStorage.setItem('google_id_token', tokens.id_token);
        console.log('[OAuth] Login successful');

        return tokens.id_token;
    } catch (e) {
        console.error('[OAuth] Token exchange failed:', e);
        return null;
    }
}

export function getStoredIdToken() {
    return localStorage.getItem('google_id_token');
}

export function clearStoredTokens() {
    localStorage.removeItem('google_id_token');
}

export function parseJWT(token) {
    try {
        const base64Url = token.split('.')[1];
        const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
        const jsonPayload = decodeURIComponent(
            atob(base64)
                .split('')
                .map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
                .join('')
        );
        return JSON.parse(jsonPayload);
    } catch (e) {
        console.error('[OAuth] Failed to parse JWT:', e);
        return null;
    }
}
