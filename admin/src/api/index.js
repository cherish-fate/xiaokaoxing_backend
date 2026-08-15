const TOKEN_KEY = 'xkx_admin_token'
const USER_KEY = 'xkx_admin_user'

export function getToken() {
  return localStorage.getItem(TOKEN_KEY) || ''
}

export function setToken(token) {
  localStorage.setItem(TOKEN_KEY, token)
}

export function getAdminUser() {
  try {
    return JSON.parse(localStorage.getItem(USER_KEY) || 'null')
  } catch {
    return null
  }
}

export function setAdminUser(user) {
  localStorage.setItem(USER_KEY, JSON.stringify(user))
}

export function clearAuth() {
  localStorage.removeItem(TOKEN_KEY)
  localStorage.removeItem(USER_KEY)
}

export async function api(path, { method = 'GET', body, params, auth = true } = {}) {
  let url = '/api' + path
  if (params) {
    const qs = new URLSearchParams()
    Object.keys(params).forEach((key) => {
      if (params[key] !== undefined && params[key] !== null && params[key] !== '') {
        qs.append(key, params[key])
      }
    })
    const query = qs.toString()
    if (query) url += '?' + query
  }

  const headers = { 'Content-Type': 'application/json' }
  if (auth) {
    const token = getToken()
    if (token) headers.Authorization = `Bearer ${token}`
  }

  const res = await fetch(url, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body)
  })

  let data
  try {
    data = await res.json()
  } catch {
    data = { code: res.status, message: res.statusText, data: null }
  }

  if (res.status === 401 && auth && !path.includes('/admin/login')) {
    clearAuth()
    if (window.location.hash !== '#/login') {
      window.location.hash = '#/login'
    }
  }

  if (!res.ok || (data.code !== undefined && data.code !== 200)) {
    const err = new Error(data.message || '请求失败')
    err.data = data
    throw err
  }

  return data.data
}