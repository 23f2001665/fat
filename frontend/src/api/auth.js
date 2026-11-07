import client from './client'

export async function login(username, password) {
  const { data } = await client.post('/auth/login', { username, password })
  return data.user
}

export async function register(name, username, password) {
  const { data } = await client.post('/auth/register', { name, username, password })
  return data.user
}

export async function logout() {
  await client.post('/auth/logout')
}

export async function getCurrentUser() {
  const { data } = await client.get('/auth/me')
  return data.user
}

export async function changePassword(currentPassword, newPassword) {
  await client.put('/auth/password', {
    current_password: currentPassword,
    new_password: newPassword,
  })
}
