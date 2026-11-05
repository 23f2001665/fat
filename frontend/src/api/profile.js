import client from './client'

export async function getProfile() {
  const { data } = await client.get('/profile')
  return data
}

export async function updateProfile(payload) {
  const { data } = await client.put('/profile', payload)
  return data.user
}

export async function getMeasurements() {
  const { data } = await client.get('/profile/measurements')
  return data
}

export async function addMeasurement(payload) {
  const { data } = await client.post('/profile/measurements', payload)
  return data.measurement
}

export async function getMetricSeries({ metric, period = 'all', start, end }) {
  const { data } = await client.get('/profile/series', {
    params: { metric, period, start: start || undefined, end: end || undefined },
  })
  return data
}
