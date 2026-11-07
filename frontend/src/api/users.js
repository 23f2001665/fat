import client from './client'

export async function getUsers() {
  const { data } = await client.get('/admin/users')
  return data.users
}

export async function createUser(payload) {
  const { data } = await client.post('/admin/users', payload)
  return data.user
}

export async function updateUser(id, payload) {
  const { data } = await client.put(`/admin/users/${id}`, payload)
  return data.user
}
